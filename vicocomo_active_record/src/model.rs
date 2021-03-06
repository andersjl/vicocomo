use ::proc_macro::TokenStream;
use ::proc_macro2::Span;
use ::syn::{
    parse_quote, punctuated::Punctuated, AttrStyle, Attribute, Expr, Ident,
    Lit, LitStr, Meta, NestedMeta, Type,
};
use ::vicocomo_derive_utils::*;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ExtraInfo {
    BelongsToData,
    DatabaseTypes,
    HasManyData,
    OrderFields,
    UniqueFields,
}

#[derive(Clone, Debug)]
pub(crate) enum Order {
    Asc(u32),
    Desc(u32),
}
impl Order {
    pub(crate) fn prio(&self) -> u32 {
        match self {
            Self::Asc(p) => *p,
            Self::Desc(p) => *p,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ForKey {
    // The name attribute value
    pub(crate) assoc_name: Option<String>,
    // The name attibute or the remote type last segment, snaked
    pub(crate) assoc_snake: String,
    // The remote type primary key field
    pub(crate) remote_pk: Ident,
    // Mandatory remote type primary key field
    pub(crate) remote_pk_mand: bool,
    // The remote type full path
    pub(crate) remote_type: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct Field {
    pub(crate) id: Ident,
    pub(crate) ty: Type,
    pub(crate) col: LitStr,
    // .1 indicates whether the column is nullable, i.e. a Nul... DbType
    pub(crate) dbt: Option<(Expr, bool)>,
    pub(crate) pri: bool,
    pub(crate) uni: Option<String>,
    pub(crate) ord: Option<Order>,
    // indicates whether the field is optional. It may be nullable regardless.
    pub(crate) opt: bool,
    pub(crate) fk: Option<ForKey>,
}

#[derive(Clone, Debug)]
pub(crate) struct HasMany {
    // The name attribute value, as a String
    pub(crate) assoc_name: Option<String>,
    // The name attibute or the remote type last segment, snaked
    pub(crate) assoc_snake: String,
    // What to do when deleting self
    pub(crate) on_delete: OnDelete,
    // The name of the Remote BelongsTo association to Self
    pub(crate) remote_assoc: String,
    // The database name of the foreign key column to Self in Remote or join
    pub(crate) remote_fk_col: String,
    // The remote type full path
    pub(crate) remote_type: Type,
    // The rest differs between one- and many-to-many
    pub(crate) many_to_many: Option<ManyToMany>,
}

// relevant only for a many-to-many association
#[derive(Clone, Debug)]
pub(crate) struct ManyToMany {
    // The database name of the join table
    pub(crate) join_table_name: String,
    // The database name of the join table foreign key column to Remote
    pub(crate) join_fk_col: String,
    // The remote type primary key field
    pub(crate) remote_pk: Ident,
    // Mandatory remote type primary key field
    pub(crate) remote_pk_mand: bool,
    // The remote type primary key field
    pub(crate) remote_pk_col: String,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum OnDelete {
    Cascade,
    Forget,
    Restrict,
}

#[derive(Clone, Debug)]
pub(crate) struct Model {
    // The struct that derives
    pub(crate) struct_id: Ident,
    // Database table name
    pub(crate) table_name: String,
    pub(crate) has_many: Vec<HasMany>,
    // indicates presence of the vicocomo_before_delete attribute
    pub(crate) before_delete: bool,
    // indicates presence of the vicocomo_before_save attribute
    pub(crate) before_save: bool,
    pub(crate) fields: Vec<Field>,
}

impl Model {
    // public methods without receiver - - - - - - - - - - - - - - - - - - - -

    pub(crate) fn new(input: TokenStream, compute: Vec<ExtraInfo>) -> Self {
        use ::case::CaseExt;
        use ::regex::Regex;
        use ::syn::{
            parse, Data::Struct, DeriveInput, Fields::Named, FieldsNamed,
        };

        const EXPECT_BELONGS_TO_ERROR: &'static str =
            "expected #[vicocomo_belongs_to( ... )]";
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
        let before_delete: bool = attrs
            .iter()
            .any(|a| a.path.is_ident("vicocomo_before_delete"));
        let before_save: bool = attrs
            .iter()
            .any(|a| a.path.is_ident("vicocomo_before_save"));
        let has_many: Vec<HasMany> =
            if compute.contains(&ExtraInfo::HasManyData) {
                Self::get_has_many(attrs, &struct_id.to_string())
            } else {
                Vec::new()
            };
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
            let mut fk = None;
            for attr in field.attrs {
                match attr.style {
                    AttrStyle::Inner(_) => continue,
                    _ => (),
                }
                let attr_id =
                    attr.path.segments.first().unwrap().ident.clone();
                match attr_id.to_string().as_str() {
                    "vicocomo_belongs_to"
                        if compute.contains(&ExtraInfo::BelongsToData) =>
                    {
                        let field_name = id.to_string();
                        let mut assoc_name: Option<String> = None;
                        let mut remote_pk =
                            Ident::new("id", Span::call_site());
                        let mut remote_pk_mand = false;
                        let mut remote_type_string = Regex::new(r"_id$")
                            .unwrap()
                            .find(&field_name)
                            .and_then(|mat| {
                                Some(
                                    field_name[..mat.start()]
                                        .to_camel()
                                        .to_string(),
                                )
                            });
                        match attr
                            .parse_meta()
                            .expect(EXPECT_BELONGS_TO_ERROR)
                        {
                            Meta::List(list) => {
                                for entry in list.nested.iter() {
                                    match entry {
                                        NestedMeta::Meta(nested) => {
                                            match nested {
Meta::NameValue(n_v) => {
    match n_v.path.get_ident().unwrap().to_string().as_str() {
        "name" => match &n_v.lit {
            Lit::Str(s) => assoc_name = Some(s.value()),
            _ => panic!(EXPECT_BELONGS_TO_ERROR),
        }
        "remote_pk" => match &n_v.lit {
            Lit::Str(lit_str) => {
                let given = lit_str.value();
                let pk_string;
                match Regex::new(r"\s+mandatory$").unwrap().find(&given) {
                    Some(mat) => {
                        pk_string = given[..mat.start()].to_string();
                        remote_pk_mand = true;
                    }
                    None => pk_string = given,
                }
                remote_pk = Ident::new(&pk_string, Span::call_site());
            }
            _ => panic!(EXPECT_BELONGS_TO_ERROR),
        }
        "remote_type" => match &n_v.lit {
            Lit::Str(s) => {
                remote_type_string = Some(s.value())
            }
            _ => panic!(EXPECT_BELONGS_TO_ERROR),
        }
        _ => panic!(EXPECT_BELONGS_TO_ERROR),
    }
}
_ => panic!(EXPECT_BELONGS_TO_ERROR),
                                            }
                                        }
                                        _ => panic!(EXPECT_BELONGS_TO_ERROR),
                                    }
                                }
                            }
                            _ => panic!(EXPECT_BELONGS_TO_ERROR),
                        }
                        let (remote_type, rem_type_str) =
                            Self::remote_type(&remote_type_string.unwrap());
                        let assoc_snake = assoc_name
                            .as_ref()
                            .unwrap_or(&rem_type_str)
                            .to_snake();
                        fk = Some(ForKey {
                            assoc_name,
                            assoc_snake,
                            remote_pk,
                            remote_pk_mand,
                            remote_type,
                        });
                    }
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
                use ::lazy_static::lazy_static;
                use ::quote::format_ident;
                use ::std::{collections::HashMap, fs::read_to_string};
                lazy_static! {
                    pub static ref DB_TYPES: HashMap<String, (String, bool)> = {
                        let mut map = HashMap::new();
                        let mut db_types = "
                            f32 => Float,
                            f64 => Float,
                            bool => Int,
                            i32 => Int,
                            i64 => Int,
                            u32 => Int,
                            u64 => Int,
                            usize => Int,
                            NaiveDate => Int,
                            NaiveDateTime => Int,
                            NaiveTime => Int,
                            String => Text,
                            "
                        .to_string();
                        db_types.extend(
                            read_to_string("config/db-types.cfg")
                                .unwrap_or_else(|_| String::new())
                                .chars(),
                        );
                        for defs in db_types.split(',').map(|typ_var| {
                            let mut typ_var = typ_var.split("=>");
                            let typ_str = typ_var.next().unwrap().trim();
                            typ_var.next().map(|s| {
                                let var_str = s.trim();
                                (
                                    format_ident!("{}", typ_str),
                                    format!(
                                        "::vicocomo::DbType::{}",
                                        var_str,
                                    ),
                                    format!(
                                        "::vicocomo::DbType::Nul{}",
                                        var_str,
                                    ),
                                )
                            })
                        }) {
                            defs.map(|(typ_id, var_str, nul_str)| {
                                let typ: Type = parse_quote!(#typ_id);
                                let opt: Type = parse_quote!(Option<#typ_id>);
                                map.insert(
                                    tokens_to_string(&typ),
                                    (var_str, false),
                                );
                                map.insert(
                                    tokens_to_string(&opt),
                                    (nul_str, true),
                                );
                            });
                        }
                        map
                    };
                }
                let type_string = tokens_to_string(if opt {
                    Self::strip_option(&field.ty)
                } else {
                    &field.ty
                });
                let db_type = DB_TYPES.get(&type_string);
                dbt = db_type
                    .map(|(dbt_str, nullable)| {
                        let parsed = ::syn::parse_macro_input::parse::<Expr>(
                            dbt_str.parse::<TokenStream>().unwrap(),
                        );
                        (parsed.unwrap(), *nullable)
                    })
                    .or_else(|| {
                        panic!(
                        "Type {} currently not allowed in a vicocomo Active \
                            Record model",
                        type_string,
                    )
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
                fk,
            });
        }

        Self {
            struct_id,
            table_name,
            has_many,
            before_delete,
            before_save,
            fields,
        }
    }

    pub(crate) fn placeholders_expr(row_cnt: Expr, col_cnt: Expr) -> Expr {
        parse_quote!(
            (0..#row_cnt)
                .map(|row_ix| {
                    format!(
                        "({})",
                        (0..#col_cnt)
                            .map(|col_ix| {
                                format!(
                                    "${}",
                                    1 + #col_cnt * row_ix + col_ix,
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", "),
                    )
                }).collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub(crate) fn check_row_count_expr(
        query: &str,
        actual: Expr,
        expected: Expr,
    ) -> Expr {
        let error_msg = LitStr::new(
            format!("{} {{}} records, expected {{}}", query).as_str(),
            Span::call_site(),
        );
        parse_quote!(
            if #actual != #expected {
                return Err(::vicocomo::Error::database(
                    None,
                    &format!(#error_msg, #actual, #expected),
                ));
            }
        )
    }

    pub(crate) fn strip_option<'a>(ty: &'a Type) -> &'a Type {
        use ::syn::{GenericArgument, PathArguments::AngleBracketed};
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

    // public methods with receiver  - - - - - - - - - - - - - - - - - - - - -

    pub(crate) fn belongs_to_fields(&self) -> Vec<&Field> {
        self.fields.iter().filter(|f| f.fk.is_some()).collect()
    }

    pub(crate) fn cols(&self) -> Vec<String> {
        self.fields.iter().map(|f| f.col.value()).collect()
    }

    pub(crate) fn db_types(&self) -> Vec<&Expr> {
        self.fields
            .iter()
            .map(|f| &f.dbt.as_ref().unwrap().0)
            .collect()
    }

    pub(crate) fn default_order(&self) -> String {
        if self.order_fields().is_empty() {
            String::new()
        } else {
            format!(
                "ORDER BY {}",
                self.order_fields()
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
    pub(crate) fn find_sql(&self, uni_cols: &[String]) -> String {
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

    pub(crate) fn order_fields(&self) -> Vec<&Field> {
        let mut result = self
            .fields
            .iter()
            .filter(|f| f.ord.is_some())
            .collect::<Vec<_>>();
        result.sort_by_key(|f| f.ord.as_ref().unwrap().prio());
        result
    }

    pub(crate) fn pk_batch_expr(&self, batch_name: &str) -> Expr {
        let pk_len = self.fields.iter().filter(|f| f.pri).count();
        let batch: Ident = Ident::new(batch_name, Span::call_site());
        match pk_len {
            0 => panic!("missing primary key field"),
            1 => parse_quote!(
                &#batch
                    .iter()
                    .map(|foo| (*foo).clone().into())
                    .collect::<Vec<_>>()[..]
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

    pub(crate) fn pk_fields(&self) -> Vec<&Field> {
        self.fields.iter().filter(|f| f.pri).collect()
    }

    pub(crate) fn pk_select(&self) -> LitStr {
        LitStr::new(
            &self
                .pk_fields()
                .iter()
                .enumerate()
                .fold(Vec::new(), |mut cols, (ix, pk)| {
                    cols.push(format!("{} = ${}", pk.col.value(), ix + 1));
                    cols
                })
                .join(" AND "),
            Span::call_site(),
        )
    }

    // the returned expression evaluates to Option<PkType>
    pub(crate) fn expr_to_pk_tuple(&self, expr: Expr) -> Expr {
        let pk_fields = &self.pk_fields();
        let mut exprs: Vec<Expr> = Vec::new();
        let mut pk_opts: Vec<Ident> = Vec::new();
        for pk in pk_fields {
            let id = &pk.id;
            if pk.opt {
                pk_opts.push(id.clone());
                exprs.push(parse_quote!((#expr).#id.clone().unwrap()));
            } else {
                exprs.push(parse_quote!((#expr).#id.clone()));
            }
        }
        let tuple = match pk_fields.len() {
            0 => parse_quote!(None),
            1 => exprs.drain(..1).next().unwrap(),
            _ => parse_quote!( ( #( #exprs ),* ) ),
        };
        if pk_fields.is_empty() {
            tuple
        } else if pk_opts.is_empty() {
            parse_quote!(Some(#tuple))
        } else {
            let check: Expr =
                parse_quote!( #( (#expr).#pk_opts.is_some() )&&* );
            parse_quote!(
                if #check {
                    Some(#tuple)
                } else {
                    None
                }
            )
        }
    }

    pub(crate) fn pk_type(&self) -> Type {
        Self::types_to_tuple(
            self.pk_fields()
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

    pub(crate) fn pk_values(&self) -> Expr {
        let pk_ids: Vec<&Ident> =
            self.pk_fields().iter().map(|f| &f.id).collect();
        parse_quote!(
            {
                let mut values: Vec<::vicocomo::DbValue> = Vec::new();
                #( values.push(self.#pk_ids.clone().into()); )*
                values
            }
        )
    }

    pub(crate) fn rows_to_models_expr(&self, rows: Expr) -> Expr {
        let retrieved = &self.fields;
        let ids: Vec<&Ident> = retrieved.iter().map(|f| &f.id).collect();
        let vals: Vec<Expr> = retrieved
            .iter()
            .map(|f| {
                if f.opt {
                    parse_quote!(Some(val))
                } else {
                    parse_quote!(val)
                }
            })
            .collect();
        parse_quote!(
            {
                use ::std::convert::TryInto;

                let mut error: Option<::vicocomo::Error> = None;
                let mut models = Vec::new();
                let mut rows: Vec<Vec<::vicocomo::DbValue>> = #rows;
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

    pub(crate) fn unique_fields(&self) -> Vec<Vec<&Field>> {
        use ::std::collections::HashMap;

        let mut unis: HashMap<&str, Vec<&Field>> = HashMap::new();
        for field in &self.fields {
            let uni_lbl = match &field.uni {
                Some(s) if !field.pri => Some(s.as_str()),
                _ => None,
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

    pub(crate) fn upd_db_types(&self) -> Vec<Expr> {
        self.fields
            .iter()
            .filter(|f| !f.pri)
            .map(|f| f.dbt.as_ref().unwrap().0.clone())
            .collect()
    }

    pub(crate) fn upd_fields(&self) -> Vec<&Field> {
        self.fields.iter().filter(|f| !f.pri).collect()
    }

    // private methods without receiver  - - - - - - - - - - - - - - - - - - -

    fn get_has_many(attrs: Vec<Attribute>, struct_nam: &str) -> Vec<HasMany> {
        use ::case::CaseExt;
        use ::regex::Regex;

        const EXPECT_HAS_MANY_ERROR: &'static str =
            "expected #[vicocomo_has_many( ... )]";

        let mut result = Vec::new();
        for attr in attrs {
            match attr.style {
                AttrStyle::Inner(_) => continue,
                _ => (),
            }
            let attr_id = attr.path.segments.first().unwrap().ident.clone();
            match attr_id.to_string().as_str() {
                "vicocomo_has_many" => {
                    let mut assoc_name: Option<String> = None;
                    let mut join_fk_col: Option<String> = None;
                    let mut join_table_name: Option<String> = None;
                    let mut on_delete: OnDelete = OnDelete::Restrict;
                    let mut remote_assoc: Option<String> = None;
                    let mut remote_fk_col: Option<String> = None;
                    let mut remote_pk = Ident::new("id", Span::call_site());
                    let mut remote_pk_mand = false;
                    let mut remote_pk_col: Option<String> = None;
                    let mut remote_type_string: Option<String> = None;
                    match attr.parse_meta().expect(EXPECT_HAS_MANY_ERROR) {
                        Meta::List(list) => {
                            for entry in list.nested.iter() {
                                match entry {
                                    NestedMeta::Meta(nested) => match nested {
                                        Meta::NameValue(n_v) => {
                                            match n_v
                                                .path
                                                .get_ident()
                                                .unwrap()
                                                .to_string()
                                                .as_str()
    {
        "join_fk_col" =>
        match &n_v.lit {
            Lit::Str(s) => join_fk_col = Some(s.value()),
            _ => panic!(EXPECT_HAS_MANY_ERROR),
        }
        "name" =>
        match &n_v.lit {
            Lit::Str(s) => assoc_name = Some(s.value()),
            _ => panic!(EXPECT_HAS_MANY_ERROR),
        }
        "on_delete" =>
        match &n_v.lit {
            Lit::Str(s) => {
                on_delete = match s.value().as_str() {
                    "cascade" => OnDelete::Cascade,
                    "forget" => OnDelete::Forget,
                    "restrict" => OnDelete::Restrict,
                    _ => panic!(
                        "expected \"cascade\", \"forget\", or \"restrict\""
                    ),
                }
            }
            _ => panic!(EXPECT_HAS_MANY_ERROR),
        }
        "remote_assoc" =>
        match &n_v.lit {
            Lit::Str(s) => remote_assoc = Some(s.value()),
            _ => panic!(EXPECT_HAS_MANY_ERROR),
        }
        "remote_fk_col" =>
        match &n_v.lit {
            Lit::Str(s) => remote_fk_col = Some(s.value()),
            _ => panic!(EXPECT_HAS_MANY_ERROR),
        }
        "remote_pk" =>
        match &n_v.lit {
            Lit::Str(lit_str) => {
                let given = lit_str.value();
                let pk_string;
                match Regex::new(r"\s+mandatory$").unwrap().find(&given) {
                    Some(mat) => {
                        pk_string = given[..mat.start()].to_string();
                        remote_pk_mand = true;
                    }
                    None => pk_string = given,
                }
                remote_pk = Ident::new(&pk_string, Span::call_site());
            }
            _ => panic!(EXPECT_HAS_MANY_ERROR),
        }
        "remote_pk_col" =>
        match &n_v.lit {
            Lit::Str(s) => remote_pk_col = Some(s.value()),
            _ => panic!(EXPECT_HAS_MANY_ERROR),
        }
        "remote_type" =>
        match &n_v.lit {
            Lit::Str(s) => {
                remote_type_string = Some(s.value())
            }
            _ => panic!(EXPECT_HAS_MANY_ERROR),
        }
        "join_table" =>
        match &n_v.lit {
            Lit::Str(s) => {
                join_table_name = Some(s.value())
            }
            _ => panic!(EXPECT_HAS_MANY_ERROR),
        }
        _ => panic!(EXPECT_HAS_MANY_ERROR),
    }
                                        }
                                        _ => panic!(EXPECT_HAS_MANY_ERROR),
                                    },
                                    _ => panic!(EXPECT_HAS_MANY_ERROR),
                                }
                            }
                        }
                        _ => panic!(EXPECT_HAS_MANY_ERROR),
                    }
                    let (remote_type, rem_type_str) =
                        Self::remote_type(&remote_type_string.unwrap());
                    let assoc_snake = assoc_name
                        .as_ref()
                        .unwrap_or(&rem_type_str)
                        .to_snake();
                    result.push(HasMany {
                        assoc_name,
                        assoc_snake,
                        on_delete,
                        remote_assoc: remote_assoc
                            .unwrap_or(struct_nam.to_string()),
                        remote_fk_col: remote_fk_col.unwrap_or(
                            struct_nam.to_string().to_snake() + "_id",
                        ),
                        remote_type,
                        many_to_many: join_table_name.map(|join_tab| {
                            ManyToMany {
                                join_table_name: join_tab,
                                join_fk_col: join_fk_col.unwrap_or(
                                    rem_type_str.to_snake() + "_id",
                                ),
                                remote_pk: remote_pk.clone(),
                                remote_pk_mand,
                                remote_pk_col: remote_pk_col
                                    .unwrap_or(remote_pk.to_string()),
                            }
                        }),
                    });
                }
                _ => (),
            }
        }
        result
    }

    // (type path, last segment as string)
    fn remote_type(path: &str) -> (Type, String) {
        let mut type_str = path.to_string();
        let type_vec = path.split("::").collect::<Vec<_>>();
        if type_vec.len() == 1 {
            type_str = format!("crate::models::{}", type_str);
        }
        (
            syn::parse_macro_input::parse(
                type_str.parse::<TokenStream>().unwrap(),
            )
            .unwrap(),
            type_vec.last().unwrap().to_string(),
        )
    }

    fn types_to_tuple(types: &[&Type]) -> Type {
        if 1 == types.len() {
            return types[0].clone();
        }
        let mut result: syn::TypeTuple = syn::TypeTuple {
            paren_token: syn::token::Paren {
                span: proc_macro2::Span::call_site(),
            },
            elems: Punctuated::new(),
        };
        for ty in types {
            result.elems.push((*ty).clone());
        }
        result.into()
    }
}
