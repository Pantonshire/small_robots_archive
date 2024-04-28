use chrono::{DateTime, Utc};
use libshire::strings::InliningString23;
use serde::Deserialize;

// https://docs.joinmastodon.org/entities/

#[derive(Deserialize, Debug)]
pub struct MdonStatus {
  pub id: InliningString23,
  pub created_at: DateTime<Utc>,
  pub in_reply_to_id: Option<InliningString23>,
  pub in_reply_to_acccount_id: Option<InliningString23>,
  pub uri: String,
  pub url: String,
  pub content: String,
  pub account: MdonAcct,
  pub media_attachments: Vec<MdonMedia>,
  pub tags: Vec<MdonTag>,
  pub reblog: Option<Box<MdonStatus>>,
}

#[derive(Deserialize, Debug)]
pub struct MdonAcct {
  pub id: String,
}

#[derive(Deserialize, Debug)]
pub struct MdonMedia {
  pub id: InliningString23,
  #[serde(rename = "type")]
  pub media_type: MdonMediaType,
  pub url: String,
  pub preview_url: String,
  pub description: Option<String>,
  pub blurhash: String,
}

#[derive(enumscribe::EnumDeserialize, Debug)]
#[enumscribe(rename_all = "lowercase")]
pub enum MdonMediaType {
  Image,
  Gifv,
  Video,
  Audio,
  Unkown,
  #[enumscribe(other)]
  Other(String),
}

#[derive(Deserialize, Debug)]
pub struct MdonTag {
  pub name: String,
  pub url: String,
}
