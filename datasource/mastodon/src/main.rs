mod html;
mod ident;
mod mastodon;
mod model;

use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use bytes::Bytes;
use clap::Parser;
use eyre::Context;
use reqwest::{Method, Response};
use sbbarch_parser::ParsedGroup;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;

use crate::html::{MdonHtmlDoc, MdonHtmlNode, MdonHtmlTag};
use crate::mastodon::{MdonAcct, MdonStatus};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
  #[clap(long)]
  config: PathBuf,
  #[clap(long)]
  max_id: Option<String>,
  #[clap(long)]
  since_id: Option<String>,
  #[clap(long)]
  pages: Option<u32>,
  #[clap(long, default_value_t = false)]
  dry_run: bool,
}

#[derive(Deserialize, Debug)]
struct Config {
  domain: String,
  username: String,
  database: DbConfig,
}

#[derive(Deserialize, Debug)]
struct DbConfig {
  uri: String,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
  tracing_subscriber::fmt::init();

  let args = Args::parse();

  let config = {
    let config_buf = fs::read_to_string(&args.config)
      .wrap_err_with(|| format!("failed to read {}", args.config.to_string_lossy()))?;
    toml::from_str::<Config>(&config_buf).wrap_err("failed to parse config")?
  };

  let http_client = reqwest::ClientBuilder::new()
    .connect_timeout(Duration::from_secs(5))
    .timeout(Duration::from_secs(10))
    .https_only(true)
    .build()
    .wrap_err("failed to create http client")?;

  // let sql_pool = PgPool::connect(&db_url).await
  //   .wrap_err_with(|| format!("failed to connect to database at {}", db_url))?;

  let api_url = format!("https://{}/api/v1", config.domain);

  let sbbarch_mastodon = SbbarchMastodon {
    api_url,
    username: config.username,
    http: http_client,
    // sql: sql_pool,
    max_id: args.max_id,
    since_id: args.since_id,
    pages: args.pages,
    dry_run: args.dry_run,
  };

  sbbarch_mastodon.run().await
}

struct SbbarchMastodon {
  api_url: String,
  username: String,
  http: reqwest::Client,
  // sql: PgPool,
  max_id: Option<String>,
  since_id: Option<String>,
  pages: Option<u32>,
  dry_run: bool,
}

impl SbbarchMastodon {
  async fn run(&self) -> eyre::Result<()> {
    let acct = self.lookup_user().await?;
    info!(account_id = acct.id);

    let mut max_id = self.max_id.as_deref().map(Cow::Borrowed);
    let mut pages_left = self.pages;

    'pages_loop: loop {
      if let Some(pages_left) = &mut pages_left {
        let Some(next_pages_left) = pages_left.checked_sub(1) else {
          break 'pages_loop
        };
        *pages_left = next_pages_left;
      }

      let statuses = self.fetch_user_timeline_page(&acct.id, self.since_id.as_deref(), max_id.as_deref()).await?;
      let Some(last_status) = statuses.last() else {
        break 'pages_loop
      };

      max_id = Some(Cow::Owned(last_status.id.as_ref().to_owned()));
      
      for status in statuses.iter().filter(|status| !status.content.is_empty()) {
        let doc = MdonHtmlDoc::from_html_str(&status.content, 16).unwrap();

        println!();
        println!("{}", doc);
        if let Some((new_doc, parsed_group)) = parse_robot_doc(&doc) {
          println!("{}", new_doc);
          println!("{:#?}", parsed_group);
        }
        println!();
      }
    }

    Ok(())
  }

  async fn lookup_user(&self) -> eyre::Result<MdonAcct> {
    #[derive(Serialize)]
    struct UserQuery<'a> {
      acct: &'a str,
    }

    let resp = self
      .http
      .request(Method::GET, format!("{}/accounts/lookup", self.api_url))
      .query(&UserQuery {
        acct: &self.username,
      })
      .send_get_ok_bytes()
      .await
      .wrap_err("failed to lookup user")?;

    serde_json::from_slice::<MdonAcct>(&resp).wrap_err("failed to deserialise account object")
  }

  async fn fetch_user_timeline_page(
    &self, acct_id: &str, since_id: Option<&str>, max_id: Option<&str>,
  ) -> eyre::Result<Vec<MdonStatus>> {
    #[derive(Serialize)]
    struct StatusesQuery<'a> {
      limit: u32,
      max_id: Option<&'a str>,
      since_id: Option<&'a str>,
    }

    let query = StatusesQuery {
      limit: 40,
      max_id,
      since_id,
    };

    let resp = self
      .http
      .request(
        Method::GET,
        format!(
          "https://mastodon.social/api/v1/accounts/{}/statuses",
          acct_id
        ),
      )
      .query(&query)
      .send_get_ok_bytes()
      .await
      .wrap_err("failed to get statuses")?;

    serde_json::from_slice::<Vec<MdonStatus>>(&resp).wrap_err("failed to deserialise statuses")
  }
}

fn parse_robot_doc(doc: &MdonHtmlDoc) -> Option<(MdonHtmlDoc, ParsedGroup)> {
  let Some((MdonHtmlNode::Element(first_elem), tail_elems)) = doc.roots().split_first() else {
    return None;
  };
  if !matches!(first_elem.tag(), MdonHtmlTag::P) {
    return None;
  }
  let Some((MdonHtmlNode::Text(p_text), tail_children)) = first_elem.children().split_first()
  else {
    return None;
  };

  let parsed_group = sbbarch_parser::parse_group(p_text)?;

  let mut new_children = Vec::with_capacity(first_elem.children().len());
  new_children.push(MdonHtmlNode::Text(parsed_group.body.to_owned()));
  new_children.extend(tail_children.iter().cloned());
  let new_first_elem = first_elem.clone_replace_children(new_children);

  let mut new_roots = Vec::with_capacity(doc.roots().len());
  new_roots.push(MdonHtmlNode::Element(new_first_elem));
  new_roots.extend(tail_elems.iter().cloned());
  let new_doc = MdonHtmlDoc::from_roots(new_roots);

  Some((new_doc, parsed_group))
}

trait RequestBuilderExt {
  async fn send_get_ok_bytes(self) -> reqwest::Result<Bytes>;
}

impl RequestBuilderExt for reqwest::RequestBuilder {
  async fn send_get_ok_bytes(self) -> reqwest::Result<Bytes> {
    self
      .send()
      .await
      .and_then(Response::error_for_status)?
      .bytes()
      .await
  }
}
