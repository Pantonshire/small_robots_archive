mod clone_data;
mod respond;
mod templates;
mod robots;

use std::env;
use std::error;
use std::fmt;
use std::io;

use actix_web::{
    get,
    HttpResponse,
    Responder,
    HttpServer,
    App,
    web
};
use sqlx::postgres::PgPool;
use maud::{html, PreEscaped};

use clone_data::CloneData;
use respond::{ResponseResult, MarkupResponse};
use robots::{Named, Displayable, RobotPreview, RobotFull};

const DEFAULT_BIND_ADDR: &str = "[::1]:8080";

const BIND_ADDR_VAR: &str = "BIND_ADDRESS";
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
            robot_groups.custom_alt \
        FROM robots INNER JOIN robot_groups ON robots.group_id = robot_groups.id \
        ORDER BY robots.robot_number DESC \
        LIMIT 10"
    )
    .fetch_all(&*pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?; //TODO: log error?

    Ok(templates::archive_page(
        "Small Robots Archive",
        html! {
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
    ).into())
}

//TODO: render content warnings
async fn render_all_robots(pool: PgPool, page: u32) -> ResponseResult<MarkupResponse> {
    const PAGE_SIZE: u32 = 48;

    let page = match page {
        page if page < 1 => return Err(actix_web::error::ErrorNotFound("invalid page number")),
        page => page,
    };

    let num_robots = sqlx::query!("SELECT COUNT(*) AS count FROM robots")
        .fetch_one(&pool)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .count
        .unwrap_or(0);

    let num_pages = ((num_robots - 1) / (PAGE_SIZE as i64)) + 1;

    let robots = sqlx::query_as!(
        RobotPreview,
        "SELECT \
            robot_groups.id AS group_id, robots.id AS robot_id, robots.robot_number, \
            robots.ident, robots.prefix, robots.suffix, robots.plural, \
            robot_groups.content_warning, robot_groups.image_thumb_path, robot_groups.alt, \
            robot_groups.custom_alt \
        FROM robots INNER JOIN robot_groups ON robots.group_id = robot_groups.id \
        ORDER BY robots.robot_number \
        LIMIT $1 \
        OFFSET $2",
        PAGE_SIZE as i64,
        (PAGE_SIZE * (page - 1)) as i64
    )
    .fetch_all(&pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let prev_page = if page > 1 {
        Some(format!("/all/{}", page - 1))
    } else {
        None
    };

    let next_page = if (page as i64) < num_pages {
        Some(format!("/all/{}", page + 1))
    } else {
        None
    };

    let page_control = html! {
        nav class="page_control" {
            @if let Some(prev_page) = prev_page {
                p { a class="link_text" href=(prev_page) { "Previous" } }
            } @else {
                p class="disabled" { "Previous" }
            }

            p { "Page " (page) " of " (num_pages) }
            
            @if let Some(next_page) = next_page {
                p { a class="link_text" href=(next_page) { "Next" } }
            } @else {
                p class="disabled" { "Next" }
            }
        }
    };

    Ok(templates::archive_page(
        "All robots",
        html! {
            div class="section" {
                h2 { "All robots" }
                ul class="robots_grid" {
                    @for robot in &robots {
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
            }

            div class="section" {
                (page_control)
            }
        }
    ).into())
}

#[get("/all")]
async fn all_robots(pool: CloneData<PgPool>) -> ResponseResult<MarkupResponse> {
    render_all_robots(pool.inner, 1).await
}

#[get("/all/{page}")]
async fn all_robots_paged(pool: CloneData<PgPool>, page: web::Path<u32>) -> ResponseResult<MarkupResponse> {
    render_all_robots(pool.inner, page.into_inner()).await
}

fn render_robot(robot: RobotFull) -> MarkupResponse {
    let full_name = robot.full_name();

    let tweet_link = format!("https://twitter.com/smolrobots/status/{}", robot.tweet_id);

    templates::archive_page(
        &full_name,
        html! {
            div class="section" {
                @if let Some(content_warning) = robot.content_warning.as_deref() {
                    // details open {
                    //     summary { "Content warning" }
                    //     (content_warning)
                    // }
                    p { "Content warning: " (content_warning) }
                }

                h2 class="robot_title" { span class="robot_number" { "#" (robot.robot_number) } " " (full_name) }

                div class="robot_content" {
                    @if let Some(image_resource_url) = robot.image_resource_url() {
                        div class="robot_image_full_container" {
                            a href=(tweet_link) {
                                img
                                    class="robot_image_full"
                                    src=(image_resource_url)
                                    alt=(robot.image_alt())
                                    draggable="false";
                            }
                        }
                    }

                    div class="robot_description" {
                        p {
                            (robot.body)
                        }

                        p {
                            a class="link_text" href=(tweet_link) { "Go to original Tweet" }
                        }
                    }
            }
            }
        }
    ).into()
}

#[get("/robots/{number}/{ident}")]
async fn robot_page(pool: CloneData<PgPool>, path: web::Path<(i32, String)>) -> ResponseResult<MarkupResponse> {
    let (number, ident) = path.into_inner();

    //TODO: 404 not found response
    let robot = sqlx::query_as!(
        RobotFull,
        "SELECT \
            robot_groups.id AS group_id, robots.id AS robot_id, robots.robot_number, robots.prefix, \
            robots.suffix, robots.plural, robot_groups.content_warning, robot_groups.image_path, \
            robot_groups.alt, robot_groups.custom_alt, robot_groups.body, robot_groups.tweet_id \
        FROM robots INNER JOIN robot_groups ON robots.group_id = robot_groups.id \
        WHERE (robots.robot_number, robots.ident) = ($1, $2)",
        number,
        ident
    )
    .fetch_one(&*pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(render_robot(robot))
}

#[get("/daily")]
async fn daily_robot(pool: CloneData<PgPool>) -> ResponseResult<MarkupResponse> {
    let robot = sqlx::query_as!(
        RobotFull,
        "SELECT \
            robot_groups.id AS group_id, robots.id AS robot_id, robots.robot_number, robots.prefix, \
            robots.suffix, robots.plural, robot_groups.content_warning, robot_groups.image_path, \
            robot_groups.alt, robot_groups.custom_alt, robot_groups.body, robot_groups.tweet_id \
        FROM robots INNER JOIN robot_groups ON robots.group_id = robot_groups.id \
        WHERE robots.id IN (SELECT robot_id FROM past_dailies ORDER BY posted_on DESC LIMIT 1) \
        LIMIT 1",
    )
    .fetch_one(&*pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(render_robot(robot))
}

#[get("/about")]
async fn about_page() -> MarkupResponse {
    templates::archive_page(
        "About",
        html! {
            div class="section" {
                h2 id="about" { "About this site" }
                p {
                    "This is a fan-made site archiving the robots drawn by "
                    a class="link_text" href="https://twitter.com/smolrobots" { "@smolrobots" }
                    " on Twitter! " (PreEscaped("&#129302;"))
                }
                p {
                    "It is a continuation of the wonderful "
                    a class="link_text" href="https://twitter.com/aguitarpenter" { "@aguitarpenter" }
                    "'s "
                    a class="link_text" href="https://smolrobots.snekkeren.co.uk" { "original archive site" }
                    ", so special thanks to him for starting this whole thing!!"
                }
            }
            div class="section" {
                h2 id="issues" { "Reporting issues" }
                p {
                    "If you find something wrong with this site, please "
                    a class="link_text" href="https://github.com/Pantonshire/small_robots_archive/issues/new" { "open an issue" }
                    " on Github."
                }
            }
            div class="section" {
                h2 id="setup" { "Setting up your own archive instance" }
                p {
                    "This is an open-source project; the source code can be found on "
                    a class="link_text" href="https://github.com/Pantonshire/small_robots_archive" { "Github" }
                    ". You are free to clone the repository and set up your own instance of this archive!"
                }
                p {
                    "TODO: link to guide on Github"
                }
            }
            div class="section" {
                h2 id="contact" { "Contact" }
                p {
                    "If you'd like to contact me directly, I'm "
                    a class="link_text" href="https://twitter.com/pantonshiredev" { "@PantonshireDev" }
                    " on Twitter or "
                    a class="link_text" href="https://tech.lgbt/@pantonshire" { "@pantonshire@tech.lgbt" }
                    " on Mastodon."
                }
            }
        }
    ).into()
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
            .service(all_robots)
            .service(all_robots_paged)
            .service(robot_page)
            .service(daily_robot)
            .service(about_page)
    };

    let http_server = HttpServer::new(app_factory);

    let http_server = match env::var(BIND_ADDR_VAR) {
        Ok(addrs) => {
            let mut http_server = http_server;
            for addr in addrs.split_whitespace() {
                http_server = http_server.bind(addr)?;
            }
            http_server
        },
        Err(env::VarError::NotPresent) => http_server.bind(DEFAULT_BIND_ADDR)?,
        Err(err) => return Err(err.into()),
    };

    http_server
        .run()
        .await
        .map_err(ServerError::from)
}
