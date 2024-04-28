use std::{cmp::Ordering, error, fmt, num::NonZeroU8, str};

use unicode_normalization::UnicodeNormalization;

/// Invariants:
/// - The first `name_len` bytes of `name` must be valid utf8.
#[derive(Clone, Debug)]
pub struct Ident {
  season: u8,
  num: i16,
  name_len: NonZeroU8,
  name: [u8; 16],
}

impl Ident {
  pub fn new(season: u8, num: i16, prefix: &str) -> Result<Self, IdentError> {
    let (name, name_len) = str_to_ident_name(prefix);

    let name_len = u8::try_from(name_len)
      .ok()
      .and_then(NonZeroU8::new)
      .ok_or(IdentError)?;

    // SAFETY:
    // `str_to_ident_name` returns a tuple `t` where the first `t.1` bytes of `t.0` are valid
    // utf8.
    unsafe { Ok(Self::from_raw_parts(season, num, name_len, name)) }
  }

  /// # Safety
  /// The first `name_len` bytes of `name` must be valid utf8.
  unsafe fn from_raw_parts(season: u8, num: i16, name_len: NonZeroU8, name: [u8; 16]) -> Self {
    Self {
      season,
      num,
      name_len,
      name,
    }
  }

  pub fn name(&self) -> &str {
    // SAFETY:
    // It is an invariant of `Ident` that the first `name_len` bytes of `name` are valid utf8.
    unsafe { str::from_utf8_unchecked(&self.name[..usize::from(self.name_len.get())]) }
  }

  fn decode(&self) -> DecodedIdent {
    DecodedIdent {
      season: self.season,
      num: self.num,
      name: self.name(),
    }
  }
}

impl PartialEq for Ident {
  fn eq(&self, other: &Self) -> bool {
    self.decode() == other.decode()
  }
}

impl Eq for Ident {}

impl PartialOrd for Ident {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for Ident {
  fn cmp(&self, other: &Self) -> Ordering {
    self.decode().cmp(&other.decode())
  }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct DecodedIdent<'a> {
  season: u8,
  num: i16,
  name: &'a str,
}

#[derive(Debug)]
pub struct IdentError;

impl fmt::Display for IdentError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("invalid robot id")
  }
}

impl error::Error for IdentError {}

fn str_to_ident_name<const N: usize>(s: &str) -> ([u8; N], usize) {
  let mut buf = [0u8; N];
  let mut buf_slice = &mut buf[..];
  let mut len = 0usize;

  for c in s
    .chars()
    .nfc()
    .flat_map(char::to_lowercase)
    .filter(|c| c.is_alphanumeric())
  {
    if buf_slice.len() < c.len_utf8() {
      break;
    }
    let encoded_len = c.encode_utf8(buf_slice).len();
    len += encoded_len;
    buf_slice = &mut buf_slice[encoded_len..];
  }

  (buf, len)
}
