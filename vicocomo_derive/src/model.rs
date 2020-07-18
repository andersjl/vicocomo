use proc_macro::TokenStream;
use syn::{
    punctuated::Punctuated, token::Comma, Attribute, Expr, Ident, Type,
};

#[derive(Clone, Debug)]
pub struct Order(pub Ident, pub bool);
#[derive(Clone, Debug)]
pub struct Param(pub Ident, pub Type);
#[derive(Clone, Debug)]
pub struct PkParam(pub Expr, pub Type);
#[derive(Eq, PartialEq)]
pub enum ModelField {
    NewStruct,
    PkParam,
    OrderFields,
    UniqueFields,
}
pub struct Model {
    pub table_id: Ident,
    pub struct_id: Ident,
    pub new_struct_id: Option<Ident>,
    pub pk_param: Option<PkParam>,
    pub order_fields: Vec<Order>,
    pub unique_fields: Vec<Vec<Param>>,
}
impl Model {
    pub fn new(input: TokenStream, compute: Vec<ModelField>) -> Self {
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
        let table_id =
            get_id_from_attr(&struct_id, "table_name", &attrs, |id| {
                format!("{}s", id).to_snake()
            });
        let new_struct_id: Option<Ident> =
            if compute.contains(&ModelField::NewStruct) {
                Some(get_id_from_attr(
                    &struct_id,
                    "new_struct",
                    &attrs,
                    |struct_id| format!("New{}", struct_id),
                ))
            } else {
                None
            };
        let compute_pk = compute.contains(&ModelField::PkParam);
        let compute_order = compute.contains(&ModelField::OrderFields);
        let compute_unique =
            compute.contains(&ModelField::UniqueFields) || compute_pk;
        let mut pk_param: Option<PkParam> = if compute_pk {
            Some(PkParam(parse_quote!(id), parse_quote!(i32)))
        } else {
            None
        };
        let mut order_fields = vec![];
        let mut unique_fields = vec![];
        const EXPECT_UNIQUE_ERROR: &'static str =
            "expected #[unique = \"some_string\"]";
        const EXPECT_ORDER_ERROR: &'static str =
            "expected #[order_by(<int>, <\"asc\"/\"desc\">)]";
        for field in named_fields {
            for attr in field.attrs {
                match attr.style {
                    AttrStyle::Inner(_) => continue,
                    _ => (),
                }
                let attr_id =
                    attr.path.segments.first().unwrap().ident.clone();
                match attr_id.to_string().as_str() {
                    "unique" => {
                        if !compute_unique {
                            continue;
                        };
                        let label = match attr
                            .parse_meta()
                            .expect("cannot parse unique attribute")
                        {
                            Meta::NameValue(value) => match value.lit {
                                Lit::Str(lbl) => lbl.value(),
                                _ => panic!(EXPECT_UNIQUE_ERROR),
                            },
                            _ => panic!(EXPECT_UNIQUE_ERROR),
                        };
                        let (mut fields, at_ix) = match unique_fields
                            .binary_search_by_key(
                                &label,
                                |(l, _v): &(String, Vec<_>)| l.to_string(),
                            ) {
                            Ok(ix) => (unique_fields.remove(ix).1, ix),
                            Err(ix) => (vec![], ix),
                        };
                        fields.push(Param(
                            field.ident.clone().unwrap(),
                            field.ty.clone(),
                        ));
                        unique_fields.insert(at_ix, (label, fields));
                    }
                    "order_by" => {
                        if !compute_order {
                            continue;
                        };
                        let mut precedence = 0;
                        let mut descending = false;
                        match attr
                            .parse_meta()
                            .expect("cannot parse order_by attribute")
                        {
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
                                                        descending = true
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
                        order_fields.insert(
                            match order_fields.binary_search_by_key(
                                &precedence,
                                |(p, _o)| *p,
                            ) {
                                Ok(ix) => ix,
                                Err(ix) => ix,
                            },
                            (
                                precedence,
                                Order(
                                    field.ident.clone().unwrap(),
                                    descending,
                                ),
                            ),
                        );
                    }
                    _ => (),
                }
            }
        }
        if compute_pk {
            match unique_fields.iter().find(|(l, _v)| l == "pk") {
                Some((_label, fields)) => {
                    let (arg_list, type_list): (
                        Punctuated<Expr, Comma>,
                        Punctuated<Type, Comma>,
                    ) = fields.iter().fold(
                        (Punctuated::new(), Punctuated::new()),
                        |mut acc, Param(id, ty)| {
                            acc.0.push(parse_quote!(self.#id));
                            acc.1.push(ty.clone());
                            acc
                        },
                    );
                    pk_param = Some(PkParam(
                        parse_quote!((#arg_list)),
                        parse_quote!((#type_list)),
                    ));
                }
                None => (),
            }
        }
        let order_fields =
            order_fields.drain(..).map(|(_p, o)| o).collect::<Vec<_>>();
        let unique_fields =
            unique_fields.drain(..).map(|(_l, v)| v).collect();
        Self {
            table_id,
            struct_id,
            new_struct_id,
            pk_param,
            order_fields,
            unique_fields,
        }
    }
}

fn get_id_from_attr<F>(
    struct_id: &Ident,
    attr_name: &str,
    attrs: &[Attribute],
    default: F,
) -> Ident
where
    F: Fn(&Ident) -> String,
{
    use syn::{export::Span, Lit, Meta};
    let error_msg = format!("expected #[{} = \"some_name\"", attr_name);
    let table_name =
        match attrs.iter().filter(|a| a.path.is_ident(attr_name)).last() {
            Some(attr) => match attr
                .parse_meta()
                .expect(&format!("cannot parse {} attribute", attr_name))
            {
                Meta::NameValue(value) => match value.lit {
                    Lit::Str(name) => name.value(),
                    _ => panic!(error_msg),
                },
                _ => panic!(error_msg),
            },
            None => default(struct_id),
        };
    Ident::new(&table_name, Span::call_site())
}
