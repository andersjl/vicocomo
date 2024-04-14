use vicocomo::{
    DatabaseIf, HttpReqBodyPart, HttpResponse, HttpServerIf, TemplEngIf,
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

    pub fn upload(
        _db: DatabaseIf,
        srv: HttpServerIf,
        _teng: TemplEngIf,
    ) -> HttpResponse {
        let upl_tgt = std::env::temp_dir().join("__vicocomo_target__/upltvå");
        match srv.handle_upload(
            srv.req_body()
                .parts
                .iter()
                .map(|part| {
                    let mut target = None;
                    if let HttpReqBodyPart::Uploaded {
                        name: _,
                        filename,
                        content_type: _,
                    } = part
                    {
                        if let Some(nam) = filename {
                            if nam == "filetvå" {
                                target = Some(upl_tgt.as_path());
                            }
                        }
                    }
                    target
                })
                .collect::<Vec<_>>()
                .as_slice(),
        ) {
            Ok(_) => srv.resp_redirect("/"),
            Err(e) => srv.resp_error(None, Some(e)),
        }
    }
}
