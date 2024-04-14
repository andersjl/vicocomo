//! HTTP server and session traits and structs.
//!

pub mod config;
pub mod server;

pub use config::{
    decode_url_parameter, multipart_boundary, Config, ConfigAttrVal,
    HttpDbSession, HttpHandler, HttpMethod, HttpParamVals, HttpRequest,
    HttpRequestImpl, HttpRespBody, HttpServer, HttpServerImpl, HttpSession,
    NullTemplEng, TemplEng,
};
pub use server::{
    AppConfigVal, HttpHeaderVal, HttpReqBody, HttpReqBodyPart, HttpResponse,
    HttpServerIf, HttpStatus, TemplEngIf,
};
