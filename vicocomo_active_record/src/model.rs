// TODO: -> Vec<&Field> => -> Filter
use ::proc_macro::TokenStream;
use ::proc_macro2::Span;
use ::std::collections::HashMap;
use ::syn::{
    parse_quote, punctuated::Punctuated, AttrStyle, Attribute, Expr, Ident,
    Lit, LitInt, LitStr, Meta, NestedMeta, Path, Type,
};
use ::vicocomo_derive_utils::*;

const ATTR_BELONGS_TO_ERROR: &'static str =
    "expected #[vicocomo_belongs_to( ... )]";
const ATTR_COLUMN_ERROR: &'static str =
    "expected #[vicocomo_column = \"column_name\"]";
const ATTR_DB_VALUE_ERROR: &'static str =
    "expected #[vicocomo_db_value = \"<DbValue variant as str>\"]";
const ATTR_OPTIONAL_ERROR: &'static str = "expected #[vicocomo_optional]";
const ATTR_ORDER_ERROR: &'static str =
    "expected #[vicocomo_order_by(<int>, <\"ASC\"/\"DESC\">)]";
const ATTR_PRESENCE_VLDTR_ERROR: &'static str =
    "expected #[vicocomo_presence_validator] on an Option<_> field \
    that is not #[vicocomo_optional]";
const ATTR_PRIMARY_ERROR: &'static str = "expected #[vicocomo_primary]";
const ATTR_REQUIRED_ERROR: &'static str =
    "expected #[vicocomo_required] on a field that is not nullable";
const ATTR_UNIQUE_ERROR: &'static str =
    "expected #[vicocomo_unique = \"label\"]";

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
    // The name attribute value or the remote type last segment
    pub(crate) assoc_name: String,
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
    pub(crate) dbt: DbType,
    // indictates what the field is (part of) the primary key
    pub(crate) pri: bool,
    // indicates that the field must not have a zero or empty value
    pub(crate) req: bool,
    // indictates that a presence validator should be generated
    pub(crate) pre: bool,
    pub(crate) ord: Option<Order>,
    // indicates whether the field is optional. It may be nullable regardless.
    pub(crate) opt: bool,
    pub(crate) fk: Option<ForKey>,
}

#[derive(Clone, Debug)]
pub(crate) struct HasMany {
    // The name attribute value or the remote type last segment, as a String
    pub(crate) assoc_name: String,
    // What to do when deleting self
    pub(crate) on_delete: OnDelete,
    // The name of the Remote BelongsTo or HasMany association to Self
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
    // The remote type primary key column
    pub(crate) remote_pk_col: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OnDelete {
    Cascade,
    Forget,
    Restrict,
}

#[derive(Clone, Debug)]
pub(crate) struct UniqueFieldSet {
    // The set
    pub(crate) fields: Vec<Field>,
    // find_by_...
    pub(crate) find_by_id: Ident,
    // find_equal_...
    pub(crate) find_eq_id: Ident,
    // the Expr-s unwrap optional fields in the set - beware!
    pub(crate) find_self_args: Vec<Expr>,
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
    pub(crate) uniques: Vec<UniqueFieldSet>,
}

impl Model {
    // public methods without receiver - - - - - - - - - - - - - - - - - - - -

    pub(crate) fn new(input: TokenStream) -> Self {
        use ::case::CaseExt;
        use ::regex::Regex;
        use ::syn::{
            parse, Data::Struct, DeriveInput, Fields::Named, FieldsNamed,
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
        let before_delete: bool = attrs
            .iter()
            .any(|a| a.path.is_ident("vicocomo_before_delete"));
        let before_save: bool = attrs
            .iter()
            .any(|a| a.path.is_ident("vicocomo_before_save"));
        let has_many: Vec<HasMany> =
            Self::get_has_many(attrs, &struct_id.to_string());
        let mut fields = Vec::new();
        let mut unis: HashMap<String, Vec<Field>> = HashMap::new();
        for field in named_fields {
            let mut dbt = None;
            let id = field.ident.expect("expected field identifier").clone();
            let ty = field.ty.clone();
            let mut col =
                LitStr::new(id.to_string().as_str(), Span::call_site());
            let mut pri = false;
            let mut req = false;
            let mut pre = false;
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
                    "vicocomo_belongs_to" => {
                        let field_name = id.to_string();
                        let mut assoc_name_attr: Option<String> = None;
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
                        match attr.parse_meta().expect(ATTR_BELONGS_TO_ERROR)
                        {
                            Meta::List(list) => {
                                for entry in list.nested.iter() {
                                    match entry {
                                        NestedMeta::Meta(nested) => {
                                            match nested {
Meta::NameValue(n_v) => {
    match n_v.path.get_ident().unwrap().to_string().as_str() {
        "name" => match &n_v.lit {
            Lit::Str(s) => assoc_name_attr = Some(s.value()),
            _ => panic!("{}", ATTR_BELONGS_TO_ERROR),
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
            _ => panic!("{}", ATTR_BELONGS_TO_ERROR),
        }
        "remote_type" => match &n_v.lit {
            Lit::Str(s) => {
                remote_type_string = Some(s.value())
            }
            _ => panic!("{}", ATTR_BELONGS_TO_ERROR),
        }
        _ => panic!("{}", ATTR_BELONGS_TO_ERROR),
    }
}
_ => panic!("{}", ATTR_BELONGS_TO_ERROR),
                                            }
                                        }
                                        _ => panic!("{}", ATTR_BELONGS_TO_ERROR),
                                    }
                                }
                            }
                            _ => panic!("{}", ATTR_BELONGS_TO_ERROR),
                        }
                        let (remote_type, rem_type_str) =
                            Self::remote_type(&remote_type_string.unwrap());
                        let assoc_name =
                            assoc_name_attr.unwrap_or(rem_type_str);
                        fk = Some(ForKey {
                            assoc_name,
                            remote_pk,
                            remote_pk_mand,
                            remote_type,
                        });
                    }
                    "vicocomo_column" => {
                        col =
                            match attr.parse_meta().expect(ATTR_COLUMN_ERROR)
                            {
                                Meta::NameValue(value) => match value.lit {
                                    Lit::Str(co) => co,
                                    _ => panic!("{}", ATTR_COLUMN_ERROR),
                                },
                                _ => panic!("{}", ATTR_COLUMN_ERROR),
                            };
                    }
                    "vicocomo_db_value" => {
                        dbt = match attr
                            .parse_meta()
                            .expect(ATTR_DB_VALUE_ERROR)
                        {
                            Meta::NameValue(value) => match value.lit {
                                Lit::Str(var_lit) => {
                                    match var_lit.value().as_str() {
                                        "Float" => Some(DbType::Float),
                                        "Int" => Some(DbType::Int),
                                        "Text" => Some(DbType::Text),
                                        "NulFloat" => Some(DbType::NulFloat),
                                        "NulInt" => Some(DbType::NulInt),
                                        "NulText" => Some(DbType::NulText),
                                        _ => {
                                            panic!("{}", ATTR_DB_VALUE_ERROR,)
                                        }
                                    }
                                }
                                _ => panic!("{}", ATTR_DB_VALUE_ERROR),
                            },
                            _ => panic!("{}", ATTR_DB_VALUE_ERROR),
                        };
                    }
                    "vicocomo_optional" => {
                        match attr.parse_meta().expect(ATTR_OPTIONAL_ERROR) {
                            Meta::Path(_) => opt = true,
                            _ => panic!("{}", ATTR_OPTIONAL_ERROR),
                        };
                    }
                    "vicocomo_order_by" => {
                        let mut precedence = 0;
                        let mut ascending = true;
                        match attr.parse_meta().expect(ATTR_ORDER_ERROR) {
                            Meta::List(path_list) => {
                                let mut values = path_list.nested;
                                if values.len() > 2 {
                                    panic!("{}", ATTR_ORDER_ERROR);
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
                                                        "{}",
                                                        ATTR_ORDER_ERROR,
                                                    ),
                                                }
                                            }
                                            _ => {
                                                panic!("{}", ATTR_ORDER_ERROR)
                                            }
                                        },
                                        _ => panic!("{}", ATTR_ORDER_ERROR),
                                    }
                                }
                                if values.len() == 1 {
                                    match values.pop().unwrap().into_value() {
                                        NestedMeta::Lit(lit) => match lit {
                                            Lit::Int(prec) => {
                                                precedence = prec
                                                    .base10_parse()
                                                    .expect(ATTR_ORDER_ERROR)
                                            }
                                            _ => {
                                                panic!("{}", ATTR_ORDER_ERROR)
                                            }
                                        },
                                        _ => panic!("{}", ATTR_ORDER_ERROR),
                                    }
                                }
                            }
                            Meta::Path(_) => (),
                            _ => panic!("{}", ATTR_ORDER_ERROR),
                        };
                        ord = Some(if ascending {
                            Order::Asc(precedence)
                        } else {
                            Order::Desc(precedence)
                        });
                    }
                    "vicocomo_presence_validator" => {
                        match attr
                            .parse_meta()
                            .expect(ATTR_PRESENCE_VLDTR_ERROR)
                        {
                            Meta::Path(_) => pre = true,
                            _ => panic!("{}", ATTR_PRESENCE_VLDTR_ERROR),
                        };
                    }
                    "vicocomo_primary" => {
                        match attr.parse_meta().expect(ATTR_PRIMARY_ERROR) {
                            Meta::Path(_) => {
                                pri = true;
                            }
                            _ => panic!("{}", ATTR_PRIMARY_ERROR),
                        };
                    }
                    "vicocomo_required" => {
                        match attr.parse_meta().expect(ATTR_REQUIRED_ERROR) {
                            Meta::Path(_) => req = true,
                            _ => panic!("{}", ATTR_REQUIRED_ERROR),
                        };
                    }
                    "vicocomo_unique" => {
                        let label =
                            match attr.parse_meta().expect(ATTR_UNIQUE_ERROR)
                            {
                                Meta::NameValue(value) => match value.lit {
                                    Lit::Str(lbl) => lbl.value(),
                                    _ => panic!("{}", ATTR_UNIQUE_ERROR),
                                },
                                _ => panic!("{}", ATTR_UNIQUE_ERROR),
                            };
                        uni = Some(label);
                    }
                    _ => (),
                }
            }
            use ::lazy_static::lazy_static;
            use ::quote::format_ident;
            lazy_static! {
                pub(crate) static ref DB_TYPES: HashMap<String, DbType> = {
                    let mut map = HashMap::new();
                    for (typ_str, var_str) in &[
                        ("bool", "Int"),
                        ("f32", "Float"),
                        ("f64", "Float"),
                        ("i32", "Int"),
                        ("i64", "Int"),
                        ("u32", "Int"),
                        ("u64", "Int"),
                        ("usize", "Int"),
                        ("NaiveDate", "Int"),
                        ("NaiveDateTime", "Int"),
                        ("NaiveTime", "Int"),
                        ("String", "Text"),
                    ] {
                        let typ_id = format_ident!("{}", typ_str);
                        let typ: Type = parse_quote!(#typ_id);
                        let opt: Type = parse_quote!(Option<#typ_id>);
                        map.insert(
                            tokens_to_string(&typ),
                            match *var_str {
                                "Float" => DbType::Float,
                                "Int" => DbType::Int,
                                "Text" => DbType::Text,
                                _ => panic!(),
                            },
                        );
                        map.insert(
                            tokens_to_string(&opt),
                            match *var_str {
                                "Float" => DbType::NulFloat,
                                "Int" => DbType::NulInt,
                                "Text" => DbType::NulText,
                                _ => panic!(),
                            },
                        );
                    }
                    map
                };
            }
            let type_string = tokens_to_string(if opt {
                Self::strip_option(&field.ty)
            } else {
                &field.ty
            });
            let dbt = dbt.unwrap_or_else(|| {
                *DB_TYPES.get(&type_string).unwrap_or_else(|| {
                    panic!(
                        "Type {} currently not allowed in a vicocomo \
                            Active Record model",
                        type_string,
                    )
                })
            });
            assert!(
                !pre || (dbt.nul() && !opt),
                "{}",
                ATTR_PRESENCE_VLDTR_ERROR,
            );
            assert!(!(req && dbt.nul()), "{}", ATTR_REQUIRED_ERROR,);
            let field = Field {
                id,
                ty,
                col,
                dbt,
                pri,
                req,
                pre,
                ord,
                opt,
                fk,
            };
            fields.push(field.clone());
            if let Some(s) = uni {
                if !pri {
                    match unis.get_mut(&s) {
                        Some(v) => v.push(field.clone()),
                        None => {
                            unis.insert(s, vec![field.clone()]);
                        }
                    }
                }
            }
        }
        let mut uniques = Vec::new();
        uniques.extend(unis.drain().map(|(_k, fields)| {
            let mut find_self_args: Vec<Expr> = vec![parse_quote!(db)];
            let mut uni_fld_strs = Vec::new();
            for field in &fields {
                let fld_id = &field.id;
                if field.opt {
                    find_self_args
                        .push(parse_quote!(self.#fld_id.as_ref().unwrap()));
                } else {
                    find_self_args.push(parse_quote!(&self.#fld_id));
                }
                uni_fld_strs.push(fld_id.to_string());
            }
            let uni_str = uni_fld_strs.join("_and_");
            UniqueFieldSet {
                fields,
                find_by_id: Ident::new(
                    &format!("find_by_{}", &uni_str),
                    Span::call_site(),
                ),
                find_eq_id: Ident::new(
                    &format!("find_equal_{}", &uni_str),
                    Span::call_site(),
                ),
                find_self_args,
            }
        }));

        Self {
            struct_id,
            table_name,
            has_many,
            before_delete,
            before_save,
            fields,
            uniques,
        }
    }

    pub(crate) fn col_to_field_string_map(&self) -> Expr {
        let mut col_str: Vec<Expr> = Vec::new();
        let mut fld_str: Vec<Expr> = Vec::new();
        for f in &self.fields {
            let col = &f.col;
            col_str.push(parse_quote!(#col));
            let fld = LitStr::new(&f.id.to_string(), Span::call_site());
            fld_str.push(parse_quote!(#fld));
        }
        parse_quote!({
            let mut map = ::std::collections::HashMap::new();
        #(  map.insert(#col_str.to_string(), #fld_str.to_string()); )*
            map
        })
    }

    pub(crate) fn field_none_err_expr(
        model_id: &Ident,
        field_id: &Ident,
    ) -> Expr {
        let model_lit = id_to_litstr(&model_id);
        let field_lit = id_to_litstr(field_id);
        parse_quote!(
            vicocomo::Error::Model(::vicocomo::ModelError {
                error: ::vicocomo::ModelErrorKind::Invalid,
                model: #model_lit.to_string(),
                general: None,
                field_errors: vec![
                    (#field_lit.to_string(), vec!["None".to_string()]),
                ],
                assoc_errors: Vec::new(),
            })
        )
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

    pub(crate) fn before_save_expr(&self, obj: Ident) -> Expr {
        let req_chk: Vec<Expr> = self
            .fields
            .iter()
            .filter(|f| f.req)
            .map(|f| {
                let fld_id = &f.id;
                let fld_lit =
                    LitStr::new(&fld_id.to_string(), Span::call_site());
                let struct_lit = LitStr::new(
                    &self.struct_id.to_string(),
                    Span::call_site(),
                );
                let err: Expr = parse_quote!(::vicocomo::model_error!(
                    CannotSave,
                    #struct_lit: "",
                    #fld_lit: ["required"],
                ));
                if f.dbt.text() {
                    if f.opt {
                        parse_quote!(
                            match #obj.#fld_id.as_ref() {
                                Some(val) if ::vicocomo::blacken(val)
                                    .is_empty() => Err(#err),
                                _ => Ok(()),
                            }
                        )
                    } else {
                        parse_quote!(
                            if ::vicocomo::blacken(&#obj.#fld_id)
                                .is_empty()
                            {
                                Err(#err)
                            } else {
                                Ok(())
                            }
                        )
                    }
                } else {
                    if f.opt {
                        parse_quote!(
                            match #obj.#fld_id {
                                Some(val) if val == 0 => Err(#err),
                                None => Ok(()),
                            }
                        )
                    } else {
                        parse_quote!(
                            if #obj.#fld_id == 0 {
                                Err(#err)
                            } else {
                                Ok(())
                            }
                        )
                    }
                }
            })
            .collect();
        if self.before_save {
            parse_quote!({
            #(  #req_chk?; )*
                ::vicocomo::BeforeSave::before_save(#obj, db)?;
            })
        } else {
            parse_quote!({
            #(  #req_chk?; )*
            })
        }
    }

    pub(crate) fn belongs_to_fields(&self) -> Vec<&Field> {
        self.fields.iter().filter(|f| f.fk.is_some()).collect()
    }

    pub(crate) fn cols(&self) -> Vec<String> {
        self.fields.iter().map(|f| f.col.value()).collect()
    }

    pub(crate) fn db_types(&self) -> Vec<Path> {
        self.fields.iter().map(|f| f.dbt.path()).collect()
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

    // Streamline field value to an Option<&FieldValueType> regardless of
    // whether optional or not or nullable or not.
    //
    // val is an expression yielding the field value, e.g. self.some_field.
    //
    pub(crate) fn field_value_expr(&self, fld: &Field, val: Expr) -> Expr {
        if fld.opt == fld.dbt.nul() {
            if fld.dbt.nul() {
                parse_quote!(#val.as_ref().and_then(|opt| opt.as_ref()))
            } else {
                parse_quote!(Some(&#val))
            }
        } else {
            parse_quote!(#val.as_ref())
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

    pub(crate) fn pk_batch_expr(&self, batch_name: &str) -> Option<Expr> {
        let pk_len = self.fields.iter().filter(|f| f.pri).count();
        let batch: Ident = Ident::new(batch_name, Span::call_site());
        match pk_len {
            0 => None,
            1 => Some(parse_quote!(
                &#batch
                    .iter()
                    .map(|foo| (*foo).clone().into())
                    .collect::<Vec<_>>()[..]
            )),
            _ => {
                let ix = (0..pk_len).map(|i| {
                    LitInt::new(i.to_string().as_str(), Span::call_site())
                });
                Some(parse_quote!(
                    &#batch
                        .iter()
                        .fold(
                            Vec::new(),
                            |mut all_vals, pk| {
                                #( all_vals.push((*pk).#ix.into()); )*
                                all_vals
                            }
                        )[..]
                ))
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

    pub(crate) fn pk_db_values(&self) -> Expr {
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

    // obj should evaluate to a model instance
    //
    // See value().
    //
    pub(crate) fn pk_value(&self, obj: Expr) -> Expr {
        self.value(self.pk_fields().as_slice(), obj)
    }

    pub(crate) fn presence_validator_fields(&self) -> Vec<&Field> {
        self.fields.iter().filter(|f| f.pre).collect()
    }

    // rows should be a Vec<Vec<DbValue>>
    // returns Result<Vec<struct_id>, Error>
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

    pub(crate) fn upd_db_types(&self) -> Vec<Path> {
        self.fields
            .iter()
            .filter(|f| !f.pri)
            .map(|f| f.dbt.path())
            .collect()
    }

    pub(crate) fn upd_fields(&self) -> Vec<&Field> {
        self.fields.iter().filter(|f| !f.pri).collect()
    }

    // Get the value of the fields, or None if any of them is None.
    //
    // obj should evaluate to a model instance
    //
    // The returned expression evaluates to, depending on the len() of fields:
    // 0: Option::<()>::None.
    // 1: Some(value that is unwrapped if Option), or None if the field is
    //    an Option that is None.
    // n: an option holding
    //    - Some(tuple of values that are unwrapped if Option), or
    //    - None if any of the values are None.
    //
    pub(crate) fn value(&self, fields: &[&Field], obj: Expr) -> Expr {
        match fields.len() {
            0 => parse_quote!(Option::<()>::None),
            1 => {
                let def = fields.first().unwrap().clone();
                let id = &def.id;
                if def.opt {
                    parse_quote!(#obj.#id.clone())
                } else {
                    parse_quote!(Some(#obj.#id.clone()))
                }
            }
            _ => {
                let val: Vec<Expr> = fields
                    .iter()
                    .map(|def| {
                        let id = &def.id;
                        if def.opt {
                            parse_quote!(
                                match #obj.#id {
                                    Some(val) => val.clone(),
                                    None => return None,
                                }
                            )
                        } else {
                            parse_quote!(#obj.#id.clone())
                        }
                    })
                    .collect();
                parse_quote!( Some( ( #(  #val ),* ) ) )
            }
        }
    }

    // private methods without receiver  - - - - - - - - - - - - - - - - - - -

    fn get_has_many(attrs: Vec<Attribute>, struct_nam: &str) -> Vec<HasMany> {
        use ::case::CaseExt;
        use ::regex::Regex;

        const ATTR_HAS_MANY_ERROR: &'static str =
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
                    let mut assoc_name_attr: Option<String> = None;
                    let mut join_fk_col: Option<String> = None;
                    let mut join_table_name: Option<String> = None;
                    let mut on_delete: OnDelete = OnDelete::Restrict;
                    let mut remote_assoc: Option<String> = None;
                    let mut remote_fk_col: Option<String> = None;
                    let mut remote_pk = Ident::new("id", Span::call_site());
                    let mut remote_pk_mand = false;
                    let mut remote_pk_col: Option<String> = None;
                    let mut remote_type_string: Option<String> = None;
                    match attr.parse_meta().expect(ATTR_HAS_MANY_ERROR) {
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
            _ => panic!("{}", ATTR_HAS_MANY_ERROR),
        }
        "name" =>
        match &n_v.lit {
            Lit::Str(s) => assoc_name_attr = Some(s.value()),
            _ => panic!("{}", ATTR_HAS_MANY_ERROR),
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
            _ => panic!("{}", ATTR_HAS_MANY_ERROR),
        }
        "remote_assoc" =>
        match &n_v.lit {
            Lit::Str(s) => remote_assoc = Some(s.value()),
            _ => panic!("{}", ATTR_HAS_MANY_ERROR),
        }
        "remote_fk_col" =>
        match &n_v.lit {
            Lit::Str(s) => remote_fk_col = Some(s.value()),
            _ => panic!("{}", ATTR_HAS_MANY_ERROR),
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
            _ => panic!("{}", ATTR_HAS_MANY_ERROR),
        }
        "remote_pk_col" =>
        match &n_v.lit {
            Lit::Str(s) => remote_pk_col = Some(s.value()),
            _ => panic!("{}", ATTR_HAS_MANY_ERROR),
        }
        "remote_type" =>
        match &n_v.lit {
            Lit::Str(s) => {
                remote_type_string = Some(s.value())
            }
            _ => panic!("{}", ATTR_HAS_MANY_ERROR),
        }
        "join_table" =>
        match &n_v.lit {
            Lit::Str(s) => {
                join_table_name = Some(s.value())
            }
            _ => panic!("{}", ATTR_HAS_MANY_ERROR),
        }
        _ => panic!("{}", ATTR_HAS_MANY_ERROR),
    }
                                        }
                                        _ => {
                                            panic!("{}", ATTR_HAS_MANY_ERROR)
                                        }
                                    },
                                    _ => panic!("{}", ATTR_HAS_MANY_ERROR),
                                }
                            }
                        }
                        _ => panic!("{}", ATTR_HAS_MANY_ERROR),
                    }
                    let (remote_type, rem_type_str) =
                        Self::remote_type(&remote_type_string.unwrap());
                    let assoc_name =
                        assoc_name_attr.unwrap_or(rem_type_str.clone());
                    result.push(HasMany {
                        assoc_name,
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

    // types.len()  returned type
    // 0:           ()
    // 1;           types[0]
    // n:           (types[0], ... )
    //
    fn types_to_tuple(types: &[&Type]) -> Type {
        if 1 == types.len() {
            return types[0].clone();
        }
        let mut result = ::syn::TypeTuple {
            paren_token: ::syn::token::Paren {
                span: ::proc_macro2::Span::call_site(),
            },
            elems: Punctuated::new(),
        };
        for ty in types {
            result.elems.push((*ty).clone());
        }
        result.into()
    }
}

// Mirror vicocomo::DbType
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DbType {
    Float,
    Int,
    Text,
    NulFloat,
    NulInt,
    NulText,
}

impl DbType {
    pub(crate) fn path(&self) -> Path {
        match self {
            Self::Float => parse_quote!(::vicocomo::DbType::Float),
            Self::Int => parse_quote!(::vicocomo::DbType::Int),
            Self::Text => parse_quote!(::vicocomo::DbType::Text),
            Self::NulFloat => parse_quote!(::vicocomo::DbType::NulFloat),
            Self::NulInt => parse_quote!(::vicocomo::DbType::NulInt),
            Self::NulText => parse_quote!(::vicocomo::DbType::NulText),
        }
    }

    pub(crate) fn nul(&self) -> bool {
        match self {
            Self::NulFloat | Self::NulInt | Self::NulText => true,
            _ => false,
        }
    }

    pub(crate) fn text(&self) -> bool {
        match self {
            Self::Text | Self::NulText => true,
            _ => false,
        }
    }
}
