mod clone_data;
mod respond;
mod templates;
mod robots;

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
use robots::{Named, RobotPreview};

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
    let latest = sqlx::query_as!(
        RobotPreview,
        "SELECT \
            robot_groups.id AS group_id, robots.id AS robot_id, robots.robot_number, \
            robots.ident, robots.prefix, robots.suffix, robots.plural, \
            robot_groups.content_warning, robot_groups.image_thumb_path, robot_groups.alt, \
            robot_groups.custom_alt
        FROM robots INNER JOIN robot_groups ON robots.group_id = robot_groups.id \
        ORDER BY robots.robot_number DESC \
        LIMIT 10"
    )
    .fetch_all(&*pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?; //TODO: log error?

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
                        li { a class="link_text" href="/" { "Home" } }
                        li { a class="link_text" href="/all" { "All robots" } }
                        li { a class="link_text" href="/daily" { "Robot of the day" } }
                        li { a class="link_text" href="/random" { "Random" } }
                        li { a class="link_text" href="/about" { "About" } }
                    }
                }
            }
        },

        html! {
            div class="content" {
                div class="section" {
                    p {
                        "Welcome to the Small Robots Archive, a fan-made site dedicated to all of the 
                        mechanical friends drawn by the wonderful "
                        a class="link_text" href="https://twitter.com/smolrobots" { "@smolrobots" }
                        "."
                    }

                    p {
                        "If you'd like to support "
                        a class="link_text" href="https://twitter.com/smolrobots" { "@smolrobots" }
                        ", you can:"
                    }

                    ul {
                        li { a class="link_text" href=(THH_BOOK_URL) { "Buy their book!" } }
                        li { a class="link_text" href=(THH_REDBUBBLE_URL) { "Visit their Redbubble shop!" } }
                        li { a class="link_text" href=(THH_PATREON_URL) { "Become a patron!" } }
                        li { a class="link_text" href=(THH_COMMISION_URL) { "Commission your very own small robot!!!" } }
                    }
                }

                div class="section" {
                    h2 { "Recent robots" }
                    ul class="robots_row" {
                        @for robot in &latest {
                            li class="robot_container" {
                                a href=(robot.page_link()) class="link_area" {
                                    @if let Some(image_resource_url) = robot.image_resource_url() {
                                        img
                                            src=(image_resource_url)
                                            alt=(robot.image_alt())
                                            draggable="false";
                                    } @else {
                                        img alt="Image not found";
                                    }
                                    h3 { (robot.full_name()) }
                                    h3 class="robot_number" { "#"(robot.robot_number) }
                                }
                            }
                        }
                    }

                    p {
                        a class="link_text" href="/all" { "See all robots" }
                    }
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
            .service(actix_files::Files::new("/robot_images", "./generated/robot_images"))
            .service(landing_page)
    };

    HttpServer::new(app_factory)
        .bind("[::1]:7777")?
        .run()
        .await
        .map_err(ServerError::from)
}
