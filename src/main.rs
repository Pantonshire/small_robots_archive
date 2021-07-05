mod clone_data;
mod respond;
mod templates;

use std::{env, error, fmt, io, borrow::Cow};

use actix_web::{
    get,
    HttpResponse,
    Responder,
    HttpServer,
    App,
};
use sqlx::postgres::PgPool;
use maud::html;

use clone_data::CloneData;
use respond::{ResponseResult, MarkupResponse};

const DB_URL_VAR: &str = "DATABASE_URL";

const THH_BOOK_URL: &str = "https://www.hive.co.uk/Product/Thomas-Heasman-Hunt/Small-Robots--A-collection-of-one-hundred-mostly-useful-robot-friends/24078313";
const THH_REDBUBBLE_URL: &str = "https://www.redbubble.com/people/smolrobots/shop";
const THH_PATREON_URL: &str = "https://www.patreon.com/thomasheasmanhunt/posts";
const THH_COMMISION_URL: &str = "https://docs.google.com/forms/d/e/1FAIpQLSfQBDf0no0bVolIk90sgiMTHL9PpvVwDjGh6hOegCsPe4TXZg/viewform";

#[derive(Debug)]
enum ServerError {
    DbError(Box<sqlx::Error>),
    IoError(Box<io::Error>),
    EnvError(Box<env::VarError>),
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::DbError(err) => err.fmt(f),
            Self::IoError(err) => err.fmt(f),
            Self::EnvError(err) => err.fmt(f),
        }
    }
}

impl error::Error for ServerError {}

impl From<sqlx::Error> for ServerError {
    fn from(err: sqlx::Error) -> Self {
        Self::DbError(Box::new(err))
    }
}

impl From<io::Error> for ServerError {
    fn from(err: io::Error) -> Self {
        Self::IoError(Box::new(err))
    }
}

impl From<env::VarError> for ServerError {
    fn from(err: env::VarError) -> Self {
        Self::EnvError(Box::new(err))
    }
}

#[get("/")]
async fn landing_page(pool: CloneData<PgPool>) -> ResponseResult<MarkupResponse> {
    let record = sqlx::query!("select robot_id from past_dailies order by posted_on desc limit 1")
        .fetch_optional(&*pool)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?; //TODO: log error?

    let daily_robot_text = match record {
        None => Cow::Borrowed("No daily robot"),
        Some(record) => Cow::Owned(format!("Daily robot id {}", record.robot_id)),
    };

    Ok(templates::base(
        "Small Robots Archive",

        html! {
            div class="colour" {
                div class="title_banner content" {
                    div {
                        h1 { "Small Robots Archive" }
                        h2 { "Here are some drawings of helpful small robots for you" }
                    }

                    div class="banner_image_container" {
                        img
                            class="banner_image"
                            src="/static/banner_teabot.png"
                            alt="Teabot, a little smiling robot mug of tea with caterpillar tracks";
                    }
                }
            }

            div class="nav_container content" {
                nav {
                    ul {
                        li { a href="/" { "Home" } }
                        li { a href="/" { "All robots" } }
                        li { a href="/" { "Robot of the day" } }
                        li { a href="/" { "Random" } }
                        li { a href="/" { "About" } }
                    }
                }
            }
        },

        html! {
            div class="content" {
                p {
                    "Welcome to the Small Robots Archive, a fan-made site dedicated to all of the 
                    mechanical friends drawn by the wonderful "
                    a href="https://twitter.com/smolrobots" { "@smolrobots" }
                    "."
                }

                p {
                    "If you'd like to support "
                    a href="https://twitter.com/smolrobots" { "@smolrobots" }
                    ", you can:"
                }

                ul {
                    li { a href=(THH_BOOK_URL) { "Buy their book!" } }
                    li { a href=(THH_REDBUBBLE_URL) { "Visit their Redbubble shop!" } }
                    li { a href=(THH_PATREON_URL) { "Become a patron!" } }
                    li { a href=(THH_COMMISION_URL) { "Commission your very own small robot!!!" } }
                }

                p {
                    (daily_robot_text)
                }
            }
        },

        html! {
            div class="colour" {
                div class="page_footer content" {
                    p { "Here is a footer" }
                }
            }
        }
    ).into())
}

#[actix_web::main]
async fn main() -> Result<(), ServerError> {
    #[cfg(feature = "dotenv")] {
        dotenv::dotenv().ok();
    }

    let pool = {
        let db_url = env::var(DB_URL_VAR)?;
        PgPool::connect(&db_url).await?
    };

    let app_factory = move || {
        App::new()
            .app_data(CloneData::new(pool.clone()))
            .service(actix_files::Files::new("/static", "./static"))
            .service(landing_page)
    };

    HttpServer::new(app_factory)
        .bind("[::1]:7777")?
        .run()
        .await
        .map_err(ServerError::from)
}
