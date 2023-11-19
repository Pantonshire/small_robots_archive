use actix_web::{Responder, HttpRequest, HttpResponse, HttpResponseBuilder, http::StatusCode};
use maud::Markup;

pub struct MarkupResponse {
    pub markup: Markup,
    pub status: StatusCode,
}

impl MarkupResponse {
    pub const fn new(markup: Markup, status: StatusCode) -> Self {
        Self {
            markup,
            status,
        }
    }

    pub const fn ok(markup: Markup) -> Self {
        Self::new(markup, StatusCode::OK)
    }
}

impl From<MarkupResponse> for HttpResponse {
    fn from(markup_response: MarkupResponse) -> Self {
        HttpResponseBuilder::new(markup_response.status)
            .content_type("text/html; charset=utf-8")
            .body(markup_response.markup.0)
    }
}

impl Responder for MarkupResponse {
    fn respond_to(self, _: &HttpRequest) -> HttpResponse {
        self.into()
    }
}
