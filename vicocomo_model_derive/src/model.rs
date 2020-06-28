use proc_macro::TokenStream;
use syn::{export::Span, parse_quote, Expr, Ident, LitStr, Type};
use vicocomo_derive_utils::*;

#[derive(Eq, Hash, PartialEq)]
pub enum ExtraInfo {
    DatabaseTypes,
    OrderFields,
    UniqueFields,
}

#[derive(Clone, Debug)]
pub enum Order {
    Asc(u32),
    Desc(u32),
}
impl Order {
    pub fn prio(&self) -> u32 {
        match self {
            Self::Asc(p) => *p,
            Self::Desc(p) => *p,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Field {
    pub id: Ident,
    pub ty: Type,
    pub col: LitStr,
    pub dbt: Option<Expr>,
    pub pri: bool,
    pub uni: Option<String>,
    pub ord: Option<Order>,
    pub opt: bool,
}

pub struct Model {
    pub struct_id: Ident,
    pub table_name: String,
    pub fields: Vec<Field>,
    pub all_cols: Vec<String>,
    pub all_db_types: Vec<Expr>,
    pub all_fields: Vec<Ident>,
    pub all_mand_cols: Vec<LitStr>,
    pub all_mand_fields: Vec<Ident>,
    pub all_opt_cols: Vec<LitStr>,
    pub all_opt_fields: Vec<Ident>,
    pub all_pk_cols: Vec<String>,
    pub all_pk_fields: Vec<Ident>,
    pub all_upd_cols: Vec<String>,
    pub all_upd_db_types: Vec<Expr>,
    pub pk_mand_cols: Vec<LitStr>,
    pub pk_mand_fields: Vec<Ident>,
    pub pk_opt_cols: Vec<LitStr>,
    pub pk_opt_field_names: Vec<LitStr>,
    pub pk_opt_fields: Vec<Ident>,
    pub pk_type: Type,
    pub upd_mand_cols: Vec<LitStr>,
    pub upd_mand_fields: Vec<Ident>,
    pub upd_opt_cols: Vec<LitStr>,
    pub upd_opt_fields: Vec<Ident>,
}

macro_rules! concat {
    ($result:ident, $v1:ident, $v2:ident, $v3:ident, $v4:ident $( , )?) => {
        let mut $result = $v1.clone();
        $result.extend_from_slice(&$v2[..]);
        $result.extend_from_slice(&$v3[..]);
        $result.extend_from_slice(&$v4[..]);
    };
    ($result:ident, $v1:ident, $v2:ident $( , )?) => {
        let mut $result = $v1.clone();
        $result.extend_from_slice(&$v2[..]);
    };
}

impl Model {
    pub fn order_fields(&self) -> Vec<&Field> {
        let mut to_sort = self
            .fields
            .iter()
            .filter(|f| f.ord.is_some())
            .collect::<Vec<_>>();
        to_sort.sort_by_key(|f| f.ord.as_ref().unwrap().prio());
        to_sort
    }

    pub fn unique_fields(&self) -> Vec<Vec<&Field>> {
        use std::collections::HashMap;
        let mut unis: HashMap<&str, Vec<&Field>> = HashMap::new();
        for field in &self.fields {
            let uni_lbl = match &field.uni {
                Some(s) => Some(s.as_str()),
                None => {
                    if field.pri {
                        Some("__vicocomo_primary__")
                    } else {
                        None
                    }
                }
            };
            match &uni_lbl {
                Some(s) => {
                    let key = s;
                    match unis.get_mut(key) {
                        Some(v) => v.push(&field),
                        None => {
                            unis.insert(key, vec![&field]);
                        }
                    }
                }
                None => (),
            }
        }
        unis.drain().map(|(_k, v)| v).collect()
    }

    pub fn new(input: TokenStream, compute: Vec<ExtraInfo>) -> Self {
        use case::CaseExt;
        use syn::{
            parse, AttrStyle, Data::Struct, DeriveInput, Fields::Named,
            FieldsNamed, Lit, Meta, NestedMeta,
        };
        let struct_tokens: DeriveInput = parse(input).unwrap();
        let data = struct_tokens.data;
        let mut named_fields: Option<FieldsNamed> = None;
        match data {
            Struct(data_struct) => match data_struct.fields {
                Named(fields_named) => {
                    named_fields = Some(fields_named);
                }
                _ => (),
            },
            _ => panic!("must be a struct"),
        }
        let named_fields = named_fields.expect("fields must be named").named;
        let attrs = struct_tokens.attrs;
        let struct_id = struct_tokens.ident;
        let table_name =
            get_string_from_attr(&attrs, "table_name", &struct_id, |id| {
                format!("{}s", id).to_snake()
            });
        const EXPECT_COLUMN_ERROR: &'static str =
            "expected #[vicocomo_column = \"column_name\"]";
        const EXPECT_OPTIONAL_ERROR: &'static str =
            "expected #[vicocomo_optional]";
        const EXPECT_ORDER_ERROR: &'static str =
            "expected #[vicocomo_order_by(<int>, <\"ASC\"/\"DESC\">)]";
        const EXPECT_PRIMARY_ERROR: &'static str =
            "expected #[vicocomo_primary]";
        const EXPECT_UNIQUE_ERROR: &'static str =
            "expected #[vicocomo_unique = \"label\"]";
        let mut fields = Vec::new();
        for field in named_fields {
            let id = field.ident.expect("expected field identifier").clone();
            let ty = field.ty.clone();
            let mut col =
                LitStr::new(id.to_string().as_str(), Span::call_site());
            let mut dbt = None;
            let mut pri = false;
            let mut uni = None;
            let mut ord = None;
            let mut opt = false;
            for attr in field.attrs {
                match attr.style {
                    AttrStyle::Inner(_) => continue,
                    _ => (),
                }
                let attr_id =
                    attr.path.segments.first().unwrap().ident.clone();
                match attr_id.to_string().as_str() {
                    "vicocomo_column" => {
                        col = match attr
                            .parse_meta()
                            .expect(EXPECT_COLUMN_ERROR)
                        {
                            Meta::NameValue(value) => match value.lit {
                                Lit::Str(co) => co,
                                _ => panic!(EXPECT_COLUMN_ERROR),
                            },
                            _ => panic!(EXPECT_COLUMN_ERROR),
                        };
                    }
                    "vicocomo_optional" => {
                        match attr.parse_meta().expect(EXPECT_OPTIONAL_ERROR)
                        {
                            Meta::Path(_) => opt = true,
                            _ => panic!(EXPECT_OPTIONAL_ERROR),
                        };
                    }
                    "vicocomo_order_by"
                        if compute.contains(&ExtraInfo::OrderFields) =>
                    {
                        let mut precedence = 0;
                        let mut ascending = true;
                        match attr.parse_meta().expect(EXPECT_ORDER_ERROR) {
                            Meta::List(path_list) => {
                                let mut values = path_list.nested;
                                if values.len() > 2 {
                                    panic!(EXPECT_ORDER_ERROR);
                                } else if values.len() == 2 {
                                    match values.pop().unwrap().into_value() {
                                        NestedMeta::Lit(lit) => match lit {
                                            Lit::Str(direction) => {
                                                match direction
                                                    .value()
                                                    .as_str()
                                                {
                                                    "asc" => (),
                                                    "desc" => {
                                                        ascending = false;
                                                    }
                                                    _ => panic!(
                                                        EXPECT_ORDER_ERROR
                                                    ),
                                                }
                                            }
                                            _ => panic!(EXPECT_ORDER_ERROR),
                                        },
                                        _ => panic!(EXPECT_ORDER_ERROR),
                                    }
                                }
                                if values.len() == 1 {
                                    match values.pop().unwrap().into_value() {
                                        NestedMeta::Lit(lit) => match lit {
                                            Lit::Int(prec) => {
                                                precedence = prec
                                                    .base10_parse()
                                                    .expect(
                                                        EXPECT_ORDER_ERROR,
                                                    )
                                            }
                                            _ => panic!(EXPECT_ORDER_ERROR),
                                        },
                                        _ => panic!(EXPECT_ORDER_ERROR),
                                    }
                                }
                            }
                            Meta::Path(_) => (),
                            _ => panic!(EXPECT_ORDER_ERROR),
                        };
                        ord = Some(if ascending {
                            Order::Asc(precedence)
                        } else {
                            Order::Desc(precedence)
                        });
                    }
                    "vicocomo_primary" => {
                        match attr.parse_meta().expect(EXPECT_PRIMARY_ERROR) {
                            Meta::Path(_) => pri = true,
                            _ => panic!(EXPECT_PRIMARY_ERROR),
                        };
                    }
                    "vicocomo_unique"
                        if compute.contains(&ExtraInfo::UniqueFields) =>
                    {
                        let label = match attr
                            .parse_meta()
                            .expect(EXPECT_UNIQUE_ERROR)
                        {
                            Meta::NameValue(value) => match value.lit {
                                Lit::Str(lbl) => lbl.value(),
                                _ => panic!(EXPECT_UNIQUE_ERROR),
                            },
                            _ => panic!(EXPECT_UNIQUE_ERROR),
                        };
                        uni = Some(label);
                    }
                    _ => (),
                }
            }
            if compute.contains(&ExtraInfo::DatabaseTypes) {
                let type_string = tokens_to_string(if opt {
                    Self::strip_option(&field.ty)
                } else {
                    &field.ty
                });
                dbt = Some(match type_string.as_str() {
                    "f32" | "f64" => parse_quote!(vicocomo::DbType::Float),
                    "bool" | "i32" | "i64" | "u32" | "u64" | "usize"
                    | "NaiveDate" => parse_quote!(vicocomo::DbType::Int),
                    "String" => parse_quote!(vicocomo::DbType::Text),
                    "Option < f32 >" | "Option < f64 >" => {
                        parse_quote!(vicocomo::DbType::NulFloat)
                    }
                    "Option < bool >"
                    | "Option < i32 >"
                    | "Option < i64 >"
                    | "Option < u32 >"
                    | "Option < u64 >"
                    | "Option < usize >"
                    | "Option < NaiveDate >" => {
                        parse_quote!(vicocomo::DbType::NulInt)
                    }
                    "Option < String >" => {
                        parse_quote!(vicocomo::DbType::NulText)
                    }
                    _ => panic!(
                        "Type {} currently not allowed in a vicocomo model",
                        type_string,
                    ),
                });
            }
            fields.push(Field {
                id,
                ty,
                col,
                dbt,
                pri,
                uni,
                ord,
                opt,
            });
        }
        let mut pk_mand_cols: Vec<LitStr> = Vec::new();
        let mut pk_mand_db_types: Vec<Expr> = Vec::new();
        let mut pk_mand_fields: Vec<Ident> = Vec::new();
        let mut pk_opt_cols: Vec<LitStr> = Vec::new();
        let mut pk_opt_db_types: Vec<Expr> = Vec::new();
        let mut pk_opt_field_names: Vec<LitStr> = Vec::new();
        let mut pk_opt_fields: Vec<Ident> = Vec::new();
        let mut pk_types: Vec<&Type> = Vec::new();
        let mut upd_mand_cols: Vec<LitStr> = Vec::new();
        let mut upd_mand_db_types: Vec<Expr> = Vec::new();
        let mut upd_mand_fields: Vec<Ident> = Vec::new();
        let mut upd_opt_cols: Vec<LitStr> = Vec::new();
        let mut upd_opt_db_types: Vec<Expr> = Vec::new();
        let mut upd_opt_fields: Vec<Ident> = Vec::new();
        for field in &fields {
            let col = &field.col;
            let dbt = &field.dbt;
            let id = &field.id;
            if field.opt {
                if field.pri {
                    pk_opt_cols.push(col.clone());
                    if dbt.is_some() {
                        pk_opt_db_types.push(dbt.as_ref().unwrap().clone());
                    }
                    pk_opt_field_names.push(LitStr::new(
                        id.to_string().as_str(),
                        Span::call_site(),
                    ));
                    pk_opt_fields.push(id.clone());
                    pk_types.push(&Self::strip_option(&field.ty));
                } else {
                    upd_opt_cols.push(col.clone());
                    if dbt.is_some() {
                        upd_opt_db_types.push(dbt.as_ref().unwrap().clone());
                    }
                    upd_opt_fields.push(id.clone());
                }
            } else {
                if field.pri {
                    pk_mand_cols.push(col.clone());
                    if dbt.is_some() {
                        pk_mand_db_types.push(dbt.as_ref().unwrap().clone());
                    }
                    pk_mand_fields.push(id.clone());
                    pk_types.push(&field.ty);
                } else {
                    upd_mand_cols.push(col.clone());
                    if dbt.is_some() {
                        upd_mand_db_types.push(dbt.as_ref().unwrap().clone());
                    }
                    upd_mand_fields.push(id.clone());
                }
            };
        }
        // The derive macro implementations are much simplified by always
        // having mandatory fields before optional
        concat! {
            all_cols,
            pk_mand_cols,
            upd_mand_cols,
            pk_opt_cols,
            upd_opt_cols,
        }
        let all_cols = all_cols.iter().map(|c| c.value()).collect::<Vec<_>>();
        concat! {
            all_db_types,
            pk_mand_db_types,
            upd_mand_db_types,
            pk_opt_db_types,
            upd_opt_db_types,
        }
        concat! {
            all_fields,
            pk_mand_fields,
            upd_mand_fields,
            pk_opt_fields,
            upd_opt_fields,
        }
        concat! { all_mand_cols, pk_mand_cols, upd_mand_cols }
        concat! { all_mand_fields, pk_mand_fields, upd_mand_fields }
        concat! { all_pk_cols, pk_mand_cols, pk_opt_cols }
        let all_pk_cols =
            all_pk_cols.iter().map(|c| c.value()).collect::<Vec<_>>();
        concat! { all_pk_fields, pk_mand_fields, pk_opt_fields }
        concat! { all_opt_cols, pk_opt_cols, upd_opt_cols }
        concat! { all_opt_fields, pk_opt_fields, upd_opt_fields }
        concat! { all_upd_cols, upd_mand_cols, upd_opt_cols }
        let all_upd_cols =
            all_upd_cols.iter().map(|c| c.value()).collect::<Vec<_>>();
        concat! { all_upd_db_types, upd_mand_db_types, upd_opt_db_types }
        let pk_type = Self::type_vec_to_tuple(pk_types.as_slice());
        Self {
            struct_id,
            table_name,
            fields,
            all_cols,
            all_db_types,
            all_fields,
            all_mand_cols,
            all_mand_fields,
            all_opt_cols,
            all_opt_fields,
            all_pk_cols,
            all_pk_fields,
            all_upd_cols,
            all_upd_db_types,
            pk_mand_cols,
            pk_mand_fields,
            pk_opt_cols,
            pk_opt_field_names,
            pk_opt_fields,
            pk_type,
            upd_mand_cols,
            upd_mand_fields,
            upd_opt_cols,
            upd_opt_fields,
        }
    }

    fn strip_option<'a>(ty: &'a Type) -> &'a Type {
        use syn::{GenericArgument, PathArguments::AngleBracketed};
        match ty {
            Type::Path(p) => match p.path.segments.first() {
                Some(segm) if segm.ident == "Option" => {
                    match &segm.arguments {
                        AngleBracketed(args) => match args.args.first() {
                            Some(arg) => match arg {
                                GenericArgument::Type(t) => return t,
                                _ => (),
                            },
                            _ => (),
                        },
                        _ => (),
                    }
                }
                _ => (),
            },
            _ => (),
        }
        panic!("expected Option<_>, got {:?}", ty);
    }

    fn type_vec_to_tuple(types: &[&Type]) -> Type {
        if 1 == types.len() {
            return types[0].clone();
        }
        let mut result: syn::TypeTuple = syn::TypeTuple {
            paren_token: syn::token::Paren {
                span: proc_macro2::Span::call_site(),
            },
            elems: syn::punctuated::Punctuated::new(),
        };
        for ty in types {
            result.elems.push((*ty).clone());
        }
        result.into()
    }

    // SELECT col1, col2, col3 FROM table WHERE col1 = $1 AND col3 = $2
    pub fn find_sql(&self, uni_cols: &[String]) -> String {
        format!(
            "SELECT {} FROM {} WHERE {}",
            &self.all_cols.join(", "),
            &self.table_name,
            &uni_cols
                .iter()
                .enumerate()
                .map(|(ix, col)| format!("{} = ${}", col, ix + 1))
                .collect::<Vec<_>>()
                .join(" AND "),
        )
    }

    pub fn pk_batch_expr(&self, batch_name: &str) -> Expr {
        let pk_len = self.all_pk_fields.len();
        let batch: Ident = Ident::new(batch_name, Span::call_site());
        match pk_len {
            0 => panic!("missing primary key field"),
            1 => parse_quote!(
                &#batch.iter().map(|v| (*v).into()).collect::<Vec<_>>()[..]
            ),
            _ => {
                let ixs = (0..pk_len).map(|i| {
                    syn::LitInt::new(i.to_string().as_str(), Span::call_site())
                });
                parse_quote!(
                    &#batch
                        .iter()
                        .fold(
                            Vec::new(),
                            |mut all_vals, pk| {
                                #( all_vals.push((*pk).#ixs.into()); )*
                                all_vals
                            }
                        )[..]
                )
            }
        }
    }

    pub fn pk_select(&self) -> LitStr {
        let mut pk_cols: Vec<_> = self.pk_mand_cols
            .iter()
            .enumerate()
            .map(|(ix, col)| format!("{} = ${}", col.value(), ix + 1))
            .collect();
        let mand_len = pk_cols.len();
        for (ix, col) in self.pk_opt_cols.iter().enumerate() {
            pk_cols.push(format!("{} = ${}", col.value(), ix + mand_len + 1));
        }
        LitStr::new(&pk_cols.join(" AND "), Span::call_site())
    }

    pub fn pk_self_to_tuple(&self) -> Expr {
        use syn::{punctuated::Punctuated, token::Comma};
        let mut exprs: Punctuated<Expr, Comma> = Punctuated::new();
        for mand in &self.pk_mand_fields {
            exprs.push(parse_quote!(self.#mand));
        }
        for opt in &self.pk_opt_fields {
            exprs.push(parse_quote!(self.#opt.unwrap()));
        }
        match self.all_pk_fields.len() {
            0 => panic!("missing primary key"),
            1 => exprs[0].clone(),
            _ => parse_quote!((#exprs)),
        }
    }

    pub fn pk_values(&self) -> Expr {
        let all_pk_fields = &self.all_pk_fields;
        parse_quote!(
            {
                let mut values: Vec<vicocomo::DbValue> = Vec::new();
                #( values.push(self.#all_pk_fields.clone().into()); )*
                values
            }
        )
    }

    pub fn placeholders_expr(row_cnt: Expr, col_cnt: Expr) -> Expr {
        parse_quote!(
            (0..#row_cnt)
                .map(|row_ix| {
                    format!(
                        "({})",
                        (0..#col_cnt)
                            .map(|col_ix| {
                                format!( "${}", 1 + #col_cnt * row_ix + col_ix,)
                            })
                            .collect::<Vec<_>>()
                            .join(", "),
                    )
                }).collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub fn query_err(query: &str) -> LitStr {
        LitStr::new(
            format!("{} {{}} records, expected {{}}", query).as_str(),
            Span::call_site(),
        )
    }

    pub fn rows_to_models_expr(
        rows: Expr,
        man: &[Ident],
        opt: &[Ident],
    ) -> Expr {
        parse_quote!(
            {
                let mut error: Option<vicocomo::Error> = None;
                let mut models = Vec::new();
                for mut row in #rows.drain(..) {
                    #(
                        let #man;
                        match row.drain(..1).next().unwrap().try_into() {
                            Ok(val) => #man = val,
                            Err(err) => {
                                error = Some(err);
                                break;
                            },
                        }
                    )*
                    #(
                        let #opt;
                        match row.drain(..1).next().unwrap().try_into() {
                            Ok(val) => #opt = Some(val),
                            Err(err) => {
                                error = Some(err);
                                break;
                            },
                        }
                    )*
                    models.push(Self {
                        #( #man , )*
                        #( #opt , )*
                    });
                }
                match error {
                    Some(err) => Err(err),
                    None => Ok(models),
                }
            }
        )
    }
}
