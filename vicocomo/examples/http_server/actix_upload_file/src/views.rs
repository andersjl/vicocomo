use serde::Serialize;
use serde_json::value::Value as JsonValue;
use vicocomo::{HtmlForm, HtmlInput, HttpResponse, TemplEngIf};

#[derive(Clone, Debug, HtmlForm, Serialize)]
pub struct Upload {
    errors: Vec<String>,
    #[vicocomo_html_input_type = "File"]
    pub file_field: HtmlInput<String>,
}

pub fn home(teng: TemplEngIf) -> HttpResponse {
    #[derive(Clone, Debug, Serialize)]
    struct Data {
        upload: JsonValue,
    }
    let mut upload = Upload::with_labels(None);
    upload.file_field.set_attr("multiple", None);
    upload.file_field.set_attr("id", Some("file_field-id"));
    HttpResponse::from_result(
        teng.render(
            "home",
            &Data {
                upload: upload.to_json(),
            },
        ),
        "html",
    )
}
