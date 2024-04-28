use std::error;
use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use sqlx::postgres::{PgHasArrayType, PgTypeInfo};
use sqlx::Type;

#[derive(Type, Clone, Debug)]
#[sqlx(type_name = "robot_ident")]
pub struct IdentBuf {
  pub number: i32,
  pub name: String,
}

impl IdentBuf {
  pub fn new(number: i32, name: String) -> Self {
    Self { number, name }
  }
}

impl PgHasArrayType for IdentBuf {
  fn array_type_info() -> PgTypeInfo {
    // PostgreSQL internally names array types by prefixing the type name with an underscore
    PgTypeInfo::with_name("_robot_ident")
  }
}

impl fmt::Display for IdentBuf {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}/{}", self.number, self.name)
  }
}

impl FromStr for IdentBuf {
  type Err = ParseIdentError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let (number, name) = s.split_once('/').ok_or(ParseIdentError::MissingSlash)?;

    let number = number
      .parse::<i32>()
      .map_err(ParseIdentError::InvalidNumber)?;

    Ok(IdentBuf::new(number, name.to_owned()))
  }
}

#[derive(Debug)]
pub enum ParseIdentError {
  MissingSlash,
  InvalidNumber(ParseIntError),
}

impl fmt::Display for ParseIdentError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ParseIdentError::MissingSlash => write!(f, "missing slash separator"),
      ParseIdentError::InvalidNumber(err) => err.fmt(f),
    }
  }
}

impl error::Error for ParseIdentError {}
