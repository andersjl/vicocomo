use serde::Serialize;
use serde_json::value::Value as JsonValue;
use vicocomo::{HtmlForm, HtmlInput, HttpResponse, HttpServerIf, TemplEngIf};

#[derive(Clone, Debug, HtmlForm, Serialize)]
pub struct ReadFile {
    errors: Vec<String>,
    #[vicocomo_html_input_type = "File"]
    pub file_field: HtmlInput<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PartData {
    pub content_type: String,
    pub contents: String,
}

pub fn home(teng: TemplEngIf) -> HttpResponse {
    #[derive(Clone, Debug, Serialize)]
    struct Data {
        read_file: JsonValue,
    }
    let mut read_file = ReadFile::with_labels(None);
    read_file.file_field.set_attr("multiple", None);
    read_file.file_field.set_attr("id", Some("file_field-id"));
    HttpResponse::from_result(
        teng.render(
            "home",
            &Data {
                read_file: read_file.to_json(),
            },
        ),
        "html",
    )
}

pub fn show_file(
    _srv: HttpServerIf,
    teng: TemplEngIf,
    parts: Vec<PartData>,
) -> HttpResponse {
    HttpResponse::from_result(teng.render("show_file", &parts), "html")
}
