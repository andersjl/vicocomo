use crate::views::PartData;
use vicocomo::{
    DatabaseIf, Error, HttpReqBodyPart, HttpResponse, HttpServerIf,
    TemplEngIf,
};

pub struct Test;

impl Test {
    pub fn home(
        _db: DatabaseIf,
        _srv: HttpServerIf,
        teng: TemplEngIf,
    ) -> HttpResponse {
        crate::views::home(teng)
    }

    pub fn read_file(
        _db: DatabaseIf,
        srv: HttpServerIf,
        teng: TemplEngIf,
    ) -> HttpResponse {
        let mut parts = Vec::new();
        for part in srv.req_body().parts {
            if let HttpReqBodyPart::FormData {
                headers: _,
                name: _,
                filename: _,
                content_type,
                contents,
            } = part
            {
                parts.push(PartData {
                    content_type: content_type.to_string(),
                    contents: format!("{:?}", contents),
                });
            } else {
                return srv.resp_error(
                    None,
                    Some(Error::other(
                        "expected multipart/form-data not declared upload",
                    )),
                );
            }
        }
        crate::views::show_file(srv, teng, parts)
    }
}
