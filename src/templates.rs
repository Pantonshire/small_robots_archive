use maud::{html, Markup, DOCTYPE};

pub fn base(title: &str, header: Markup, main: Markup, footer: Markup) -> Markup {
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
