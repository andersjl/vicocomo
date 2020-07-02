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
}

impl Model {

    // public methods without receiver - - - - - - - - - - - - - - - - - - - -

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

        Self {
            struct_id,
            table_name,
            fields,
        }
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

    // public methods with receiver  - - - - - - - - - - - - - - - - - - - - -

    pub fn cols(&self) -> Vec<String> {
        self.fields.iter().map(|f| f.col.value()).collect()
    }

    pub fn db_types(&self) -> Vec<&Expr> {
        self.fields.iter().map(|f| f.dbt.as_ref().unwrap()).collect()
    }

    pub fn default_order(&self) -> String {
        if self.order_fields().is_empty() {
            String::new()
        } else {
            format!(
                "ORDER BY {}",
                self
                    .order_fields()
                    .iter()
                    .map(|f| {
                        format!(
                            "{} {}",
                            f.col.value(),
                            match f.ord.as_ref().unwrap() {
                                Order::Asc(_) => "ASC",
                                Order::Desc(_) => "DESC",
                            },
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        }
    }

    // SELECT col1, col2, col3 FROM table WHERE col1 = $1 AND col3 = $2
    pub fn find_sql(&self, uni_cols: &[String]) -> String {
        format!(
            "SELECT {} FROM {} WHERE {}",
            &self
                .fields
                .iter()
                .map(|f| f.col.value())
                .collect::<Vec<_>>()
                .join(", "),
            &self.table_name,
            &uni_cols
                .iter()
                .enumerate()
                .map(|(ix, col)| format!("{} = ${}", col, ix + 1))
                .collect::<Vec<_>>()
                .join(" AND "),
        )
    }

    pub fn order_fields(&self) -> Vec<&Field> {
        let mut result = self
            .fields
            .iter()
            .filter(|f| f.ord.is_some())
            .collect::<Vec<_>>();
        result.sort_by_key(|f| f.ord.as_ref().unwrap().prio());
        result
    }

    pub fn pk_batch_expr(&self, batch_name: &str) -> Expr {
        let pk_len = self.fields.iter().filter(|f| f.pri).count();
        let batch: Ident = Ident::new(batch_name, Span::call_site());
        match pk_len {
            0 => panic!("missing primary key field"),
            1 => parse_quote!(
                &#batch.iter().map(|v| (*v).into()).collect::<Vec<_>>()[..]
            ),
            _ => {
                let ixs = (0..pk_len).map(|i| {
                    syn::LitInt::new(
                        i.to_string().as_str(),
                        Span::call_site(),
                    )
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

    pub fn pk_fields(&self) -> Vec<&Field> {
        self
            .fields
            .iter()
            .filter(|f| f.pri)
            .collect()
    }

    pub fn pk_select(&self) -> LitStr {
        LitStr::new(&self
            .pk_fields()
            .iter()
            .enumerate()
            .fold(
                Vec::new(),
                |mut cols, (ix, pk)| {
                    cols.push(format!("{} = ${}", pk.col.value(), ix + 1));
                    cols
                }
            ).join(" AND "),
            Span::call_site()
        )
    }

    // the type of the returned expression is Option<PkType>
    pub fn pk_self_to_tuple(&self) -> Expr {
        let pk_fields = &self.pk_fields();
        let mut exprs: Vec<Expr> = Vec::new();
        let mut pk_opts: Vec<Ident> = Vec::new();
        for pk in pk_fields {
            let id = &pk.id;
            if pk.opt {
                pk_opts.push(id.clone());
                exprs.push(parse_quote!(self.#id.unwrap()));
            } else {
                exprs.push(parse_quote!(self.#id));
            }
        }
        let check: Expr = parse_quote!( #( self.#pk_opts.is_some() )&&* );
        let tuple = match pk_fields.len() {
            0 => panic!("missing primary key"),
            1 => exprs.drain(..1).next().unwrap(),
            _ => parse_quote!( ( #( #exprs ),* ) ),
        };
        parse_quote!(
            if #check {
                Some(#tuple)
            } else {
                None
            }
        )
    }

    pub fn pk_type(&self) -> Type {
        Self::types_to_tuple(
            self
                .pk_fields()
                .iter()
                .map(|pk| {
                     if pk.opt {
                        &Self::strip_option(&pk.ty)
                     } else {
                        &pk.ty
                     }
                })
                .collect::<Vec<_>>()
                .as_slice(),
        )
    }

    pub fn pk_values(&self) -> Expr {
        let pk_ids: Vec<&Ident> =
            self.pk_fields().iter().map(|f| &f.id).collect();
        parse_quote!(
            {
                let mut values: Vec<vicocomo::DbValue> = Vec::new();
                #( values.push(self.#pk_ids.clone().into()); )*
                values
            }
        )
    }

    pub fn rows_to_models_expr(&self, rows: Expr) -> Expr {
        let retrieved = &self.fields;
        let ids: Vec<&Ident> = retrieved.iter().map(|f| &f.id).collect();
        let vals: Vec<Expr> = retrieved.iter().map(|f| {
            if f.opt {
                parse_quote!(Some(val))
            } else {
                parse_quote!(val)
            }
        }).collect();
        parse_quote!(
            {
                let mut error: Option<vicocomo::Error> = None;
                let mut models = Vec::new();
                let mut rows: Vec<Vec<DbValue>> = #rows;
                for mut row in rows.drain(..) {
                    #(
                        let #ids;
                        match row
                            .drain(..1)
                            .next()
                            .unwrap()
                            .try_into()
                        {
                            Ok(val) => #ids = #vals,
                            Err(err) => {
                                error = Some(err);
                                break;
                            },
                        }
                    )*
                    models.push(Self { #( #ids ),* });
                }
                match error {
                    Some(err) => Err(err),
                    None => Ok(models),
                }
            }
        )
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

    pub fn upd_db_types(&self) -> Vec<Expr> {
        self
            .fields
            .iter()
            .filter(|f| !f.pri)
            .map(|f| f.dbt.as_ref().unwrap().clone()).collect()
    }

    pub fn upd_fields(&self) -> Vec<&Field> {
        self
            .fields
            .iter()
            .filter(|f| !f.pri)
            .collect()
    }

    // private methods without receiver  - - - - - - - - - - - - - - - - - - -

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

    fn types_to_tuple(types: &[&Type]) -> Type {
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
}