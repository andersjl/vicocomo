use ::proc_macro::TokenStream;
use ::proc_macro2::Span;
use ::quote::quote;
use ::std::str::FromStr;
use ::syn::{
    parse, parse_quote, punctuated::Punctuated, token::Comma, DeriveInput,
    Expr, GenericArgument, Ident, LitStr, PathArguments, Type,
};
use ::vicocomo_derive_utils::*;

pub fn html_form_impl(input: TokenStream) -> TokenStream {
    let struct_tokens: DeriveInput = parse(input).unwrap();
    let named_fields = named_fields(&struct_tokens)
        .expect("expected struct with named fields");
    let struct_id = struct_tokens.ident;
    let struct_name = struct_id.to_string();
    let mut field_id: Vec<Ident> = Vec::new();
    let mut field_lit: Vec<LitStr> = Vec::new();
    let mut has_errors = false;
    let mut input_id: Vec<Ident> = Vec::new();
    let mut init_expr: Vec<Expr> = Vec::new();
    let mut input_lit: Vec<LitStr> = Vec::new();
    let mut json_expr: Vec<Expr> = Vec::new();
    let mut label_str: Vec<LitStr> = Vec::new();
    for field in Field::collect(&named_fields) {
        let id = field.id.clone();
        let ty = field.input_type;
        let lit = LitStr::new(&id.to_string(), Span::call_site());
        field_id.push(id.clone());
        field_lit.push(lit.clone());
        match field.input_type {
            InputType::Errors => {
                has_errors = true;
                init_expr.push(parse_quote!(::std::vec::Vec::new()));
                json_expr.push(parse_quote!(
                    ::serde_json::to_value(self.#id.clone())
                        .unwrap_or(::serde_json::json!(null))
                ));
            }
            InputType::None => {
                init_expr.push(parse_quote!(None));
                json_expr.push(parse_quote!(
                    ::serde_json::to_value(self.#id.clone())
                        .unwrap_or(::serde_json::json!(null))
                ));
            }
            _ => {
                let typ_id = ty.to_ident();
                input_id.push(id.clone());
                init_expr.push(parse_quote!(::vicocomo::HtmlInput::new(
                    ::vicocomo::InputType::#typ_id,
                    #lit,
                )));
                input_lit.push(lit.clone());
                json_expr.push(parse_quote!(self.#id.render()));
                label_str.push(LitStr::new(
                    &(String::new()
                        + &struct_name
                        + "--"
                        + &id.to_string()
                        + "--label"),
                    Span::call_site(),
                ));
            }
        }
    }
    assert!(
        has_errors,
        "#[derive(HtmlForm)] requires {} to have a field\n    \
            errors: Vec<String>",
        struct_name,
    );
    TokenStream::from(quote! {
        impl #struct_id {
            pub fn new() -> Self {
                Self {
                #(  #field_id: #init_expr, )*
                }
            }
            pub fn with_labels(prepend: Option<&str>) -> Self {
                let mut result = Self::new();
                let mut label: String;
            #(
                result.#input_id.set_label(
                    match prepend {
                        Some(p) => {
                            label = p.to_string() + "--" + #label_str;
                            &label
                        }
                        None => #label_str,
                    }
                );
            )*
                result
            }
        }
        impl ::vicocomo::HtmlForm for #struct_id {
            fn add_error(
                &mut self,
                error: &::vicocomo::Error,
                translate: &[(&str, &str)],
            ) {
                match error {
                    ::vicocomo::Error::Model(err) => {
                        self.errors
                            .push(::vicocomo::Error::format_model(&err));
                        let mut err_flds = ::std::collections::HashMap::new();
                        let kind =
                            ::vicocomo::Error::format_model_kind(err);
                        for fe in &err.field_errors {
                            err_flds.insert(
                                fe.0.clone(),
                                ::vicocomo::Error::format_field(
                                    &kind,
                                    &err.model,
                                    fe,
                                ),
                            );
                        }
                    #(
                        let mut err_fld = #input_lit;
                        if let Some(translation) =
                            translate.iter().find(|(_, fld_nam)| {
                                *fld_nam == #input_lit
                            })
                        {
                            err_fld = translation.0;
                        };
                        if let Some(mut fld_errs) = err_flds.remove(err_fld) {
                            for err in fld_errs.drain(..) {
                                self.#input_id.add_error_text(&err);
                            }
                        }
                    )*
                    }
                    ::vicocomo::Error::Database(err) => {
                        self.errors.push(::vicocomo::Error::format_database(
                            &err,
                        ));
                    }
                    ::vicocomo::Error::InvalidInput(err) => {
                        self.errors.push(::vicocomo::Error::format_error(
                            "InvalidInput",
                            err,
                        ));
                    }
                    ::vicocomo::Error::None => {
                        self.errors.push(::vicocomo::Error::format_error(
                            "None",
                            "",
                        ));
                    }
                    ::vicocomo::Error::Other(err) => {
                        self.errors.push(::vicocomo::Error::format_error(
                            "Other",
                            err,
                        ));
                    }
                    ::vicocomo::Error::Render(err) => {
                        self.errors.push(::vicocomo::Error::format_error(
                            "Render",
                            err,
                        ));
                    }
                    ::vicocomo::Error::ThisCannotHappen(err) => {
                        self.errors.push(::vicocomo::Error::format_error(
                            "ThisCannotHappen",
                            err,
                        ));
                    }
                }
            }
            fn add_error_text(&mut self, error: &str) {
                self.errors.push(error.to_string())
            }
            fn clear_errors(&mut self) {
                self.errors.clear();
            #( self.#input_id.clear_errors(); )*
            }
            fn has_errors(&self) -> bool {
                !self.errors.is_empty() #( || self.#input_id.has_errors() )*
            }
            fn is_empty(&self) -> bool {
                true
            #(  && self.#input_id.get().is_none() )*
            }
            fn iter_error(&self) -> ::std::slice::Iter<'_, String> {
                self.errors.iter()
            }
            fn prepend_error(&mut self, error: &str) {
                if self.errors.first().map(|e| e != error).unwrap_or(true) {
                    self.errors.insert(0, error.to_string());
                }
            }
            fn to_json(&self) -> ::serde_json::value::Value {
                let mut result = ::serde_json::value::Map::new();
            #(
                result.insert(#field_lit.to_string(), #json_expr);
            )*
                ::serde_json::value::Value::Object(result)
            }
            fn to_json_values(&self) -> ::serde_json::value::Value {
                let mut result = ::serde_json::value::Map::new();
            #(
                result.insert(
                    #input_lit.to_string(),
                    ::serde_json::to_value(self.#input_id.get_mult())
                        .unwrap_or_else(|_| ::serde_json::json!([])),
                );
            )*
                ::serde_json::value::Value::Object(result)
            }
            fn update(
                &mut self,
                json: &::serde_json::Value,
            ) -> Result<(), ::vicocomo::Error> {
                let inputs =
                    if let ::serde_json::value::Value::Object(obj) = json {
                        obj
                    } else {
                        self.add_error_text(&json.to_string());
                        return Err(::vicocomo::Error::invalid_input(
                            &json.to_string(),
                        ));
                    };
                let mut backup = self.clone();
                let mut error = false;
            #(
                if let Some(val) = inputs.get(#input_lit) {
                    if let Err(_) = self.#input_id.update(val) {
                        error = true;
                    }
                }
            )*
                if error {
                #(
                    self.#input_id.set_mult(
                        backup.#input_id.get_mult().as_slice(),
                    );
                )*
                    self.prepend_error(&json.to_string());
                    self.prepend_error("update");
                    Err(::vicocomo::Error::invalid_input("update"))
                } else {
                    Ok(())
                }
            }
        }
        impl ::std::default::Default for #struct_id {
            fn default() -> Self {
                Self::new()
            }
        }
    })
}

#[derive(Debug)]
struct Field {
    id: Ident,
    input_type: InputType,
}

impl Field {
    fn collect(fields: &Punctuated<syn::Field, Comma>) -> Vec<Self> {
        let mut result: Vec<Self> = Vec::new();
        for field in fields {
            let id = field
                .ident
                .as_ref()
                .expect("expected field identifier")
                .clone();
            let default_type = InputType::from_field(&id, &field.ty);
            result.push(Self {
                id: id.clone(),
                input_type: match default_type {
                    InputType::Errors | InputType::None => default_type,
                    _ => {
                        let attr_type = get_string_from_attr(
                            &field.attrs,
                            "html_input_type",
                            &id,
                            |_| String::new(),
                        );
                        if attr_type.is_empty() {
                            default_type.clone()
                        } else {
                            attr_type.parse().unwrap()
                        }
                    }
                },
            });
        }
        result
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputType {
    Checkbox,
    Date,
    Email,
    Errors,
    Hidden,
    None,
    Number,
    Password,
    Radio,
    Range,
    Search,
    Select,
    SelectMult,
    Text,
    Textarea,
    Url,
}

impl InputType {
    fn from_field(field_id: &Ident, field_type: &Type) -> Self {
        if field_id.to_string() == "errors"
            && *field_type == parse_quote!(Vec<String>)
        {
            return InputType::Errors;
        }
        if let Type::Path(p) = field_type {
            let last = p.path.segments.last();
            if last.is_none() {
                return InputType::None;
            }
            let segm = last.unwrap();
            if segm.ident.to_string() != "HtmlInput" {
                return InputType::None;
            }
            let args;
            if let PathArguments::AngleBracketed(a) = &segm.arguments {
                args = a;
            } else {
                return InputType::None;
            }
            let arg = args.args.first();
            if arg.is_none() {
                return InputType::None;
            }
            if let GenericArgument::Type(rust_type) = arg.unwrap() {
                if let Type::Path(p) = rust_type {
                    let segm = p.path.segments.last();
                    if segm.is_none() {
                        return InputType::None;
                    }
                    match segm.unwrap().ident.to_string().as_str() {
                        "f32" | "f64" | "i8" | "i16" | "i32" | "i64"
                        | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
                        | "u128" | "usize" => Self::Number,
                        "String" => Self::Text,
                        "NaiveDate" => Self::Date,
                        _ => InputType::None,
                    }
                } else {
                    InputType::None
                }
            } else {
                InputType::None
            }
        } else {
            InputType::None
        }
    }

    pub fn to_ident(self) -> Ident {
        match self {
            Self::Checkbox => parse_quote!(Checkbox),
            Self::Date => parse_quote!(Date),
            Self::Email => parse_quote!(Email),
            Self::Errors => parse_quote!(Errors),
            Self::Hidden => parse_quote!(Hidden),
            Self::None => parse_quote!(None),
            Self::Number => parse_quote!(Number),
            Self::Password => parse_quote!(Password),
            Self::Radio => parse_quote!(Radio),
            Self::Range => parse_quote!(Range),
            Self::Search => parse_quote!(Search),
            Self::Select => parse_quote!(Select),
            Self::SelectMult => parse_quote!(SelectMult),
            Self::Text => parse_quote!(Text),
            Self::Textarea => parse_quote!(Textarea),
            Self::Url => parse_quote!(Url),
        }
    }
}

impl FromStr for InputType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Checkbox" => Ok(Self::Checkbox),
            "Date" => Ok(Self::Date),
            "Email" => Ok(Self::Email),
            "Hidden" => Ok(Self::Hidden),
            "Number" => Ok(Self::Number),
            "Password" => Ok(Self::Password),
            "Radio" => Ok(Self::Radio),
            "Range" => Ok(Self::Range),
            "Search" => Ok(Self::Search),
            "Select" => Ok(Self::Select),
            "SelectMult" => Ok(Self::SelectMult),
            "Text" => Ok(Self::Text),
            "Textarea" => Ok(Self::Textarea),
            "Url" => Ok(Self::Url),
            _ => Err(format!("{} is not an InputValue variant", s)),
        }
    }
}
