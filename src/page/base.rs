use maud::{DOCTYPE, Markup, html};

/// Returns a basic page whose body consists of three sections: header, main and footer,
/// in that order.
pub fn base(title: &str, header: Markup, main: Markup, footer: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                link rel="stylesheet" href="/static/style/main.css";
                title { (title) }
            }

            body {
                header {
                    (header)
                }

                main {
                    (main)
                }

                footer {
                    (footer)
                }
            }
        }
    }
}

/// Returns a page with the default header and footer.
pub fn archive_page(title: &str, content: Markup) -> Markup {
    base(title, header(), html! { div class="content" { (content) } }, footer())
}

/// The default header, containing a navigation menu and search bar.
pub fn header() -> Markup {
    html! {
        div class="colour" {
            div class="title_banner content" {
                h1 { "Small Robots Archive" }
                h2 { "Here are some drawings of helpful small robots for you" }
            }
        }

        div class="nav_container content" {
            nav class="site_nav" {
                ul {
                    li { a class="link_text" href="/" { "Home" } }
                    li { a class="link_text" href="/all" { "All robots" } }
                    li { a class="link_text" href="/daily" { "Robot of the day" } }
                    li { a class="link_text" href="/random" { "Random" } }
                    li { a class="link_text" href="/about" { "About" } }
                }

                //TODO: submit button
                form class="search_bar_container" method="get" action="/search" {
                    input
                        class="search_bar"
                        name="query"
                        type="text"
                        placeholder="Search...";
                }
            }
        }
    }
}

/// The default footer, containing some information about the site.
pub fn footer() -> Markup {
    html! {
        div class="colour" {
            div class="page_footer content" {
                p {
                    "The Small Robots Archive is an open-source project. To report an issue or contribute, go to "
                    a class="link_text_light" href="https://github.com/Pantonshire/small_robots_archive" { "Pantonshire/small_robots_archive" }
                    " on GitHub."
                }
            }
        }
    }
}
