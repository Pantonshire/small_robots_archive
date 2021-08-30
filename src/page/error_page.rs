use actix_web::http::StatusCode;
use maud::{html, Markup};

use super::base;

pub fn error_page(status: StatusCode) -> Markup {
    let error_string = format!(
        "{} {}",
        status.as_str(),
        status.canonical_reason().unwrap_or("Unknown Error"),
    );

    let message = error_message(status);

    base::archive_page(
        &error_string,
        html! {
            div class="section error_container" {
                h1 class="error_name" { (error_string) }
                @if let Some(message) = message {
                    p class="error_message" { (message) }
                }
            }
        }
    )
}

fn error_message(status: StatusCode) -> Option<&'static str> {
    match status {
        StatusCode::BAD_REQUEST => Some("We don't understand that request"),
        StatusCode::NOT_FOUND => Some("We couldn't find that page"),
        StatusCode::INTERNAL_SERVER_ERROR => Some("Something went wrong on our end"),
        _ => None,
    }
}
