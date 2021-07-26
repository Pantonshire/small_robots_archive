use maud::{html, Markup, DOCTYPE};

pub(crate) fn base(title: &str, header: Markup, main: Markup, footer: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
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

pub(crate) fn archive_page(title: &str, content: Markup) -> Markup {
    base(title, header(), html! { div class="content" { (content) } }, footer())
}

pub(crate) fn header() -> Markup {
    html! {
        div class="colour" {
            div class="title_banner content" {
                h1 { "Small Robots Archive" }
                h2 { "Here are some drawings of helpful small robots for you" }
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
    }
}

pub(crate) fn footer() -> Markup {
    html! {
        div class="colour" {
            div class="page_footer content" {
                p { "Here is a footer" }
            }
        }
    }
}
