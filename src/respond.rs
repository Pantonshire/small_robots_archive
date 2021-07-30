use actix_web::{Responder, HttpRequest, HttpResponse};
use maud::Markup;

pub(crate) type ResponseResult<T> = Result<T, actix_web::Error>;

pub(crate) struct MarkupResponse(pub(crate) Markup);

impl From<Markup> for MarkupResponse {
    fn from(markup: Markup) -> Self {
        Self(markup)
    }
}

impl Responder for MarkupResponse {
    fn respond_to(self, _: &HttpRequest) -> HttpResponse {
        HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(self.0.0)
    }
}
