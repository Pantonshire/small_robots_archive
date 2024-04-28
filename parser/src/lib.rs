use std::borrow::Cow;
use std::convert::TryFrom;
use std::ops::RangeInclusive;
use std::sync::OnceLock;

use regex::Regex;
use unidecode::unidecode;

use sbbarch_common::IdentBuf;

/// The name and number of a single robot.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Robot<'a> {
  pub number: i32,
  pub name: RobotName<'a>,
}

impl<'a> Robot<'a> {
  pub fn ident(&self) -> IdentBuf {
    IdentBuf {
      number: self.number,
      name: self.name.ident(),
    }
  }
}

/// The components of the name of a robot.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct RobotName<'a> {
  /// The portion of the robot's name before "bot".
  ///
  /// For example:
  /// - `"Teabot"` => `"Tea"`
  /// - `"Mischiefbots"` => `"Mischief"`
  /// - `"R.O.B.O.T.S"` => `"R.O."`
  pub prefix: Cow<'a, str>,

  /// The "bot" portion of the robot's name, made singular.
  /// For example:
  /// - `"Teabot"` => `"bot"`
  /// - `"Mischiefbots"` => `"bot"`
  /// - `"R.O.B.O.T.S"` => `"B.O.T"`
  pub suffix: Cow<'a, str>,

  /// The plural marker of the robot's name, if present.
  /// For example:
  /// - `"Teabot"` => `None`
  /// - `"Mischiefbots"` => `Some("s")`
  /// - `"R.O.B.O.T.S"` => `Some(".S")`
  pub plural: Option<Cow<'a, str>>,
}

/// The result of parsing a robot tweet.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ParsedGroup<'a> {
  /// All of the names and numbers of the robots found in the robot tweet.
  pub robots: Vec<Robot<'a>>,

  /// The body text of the robot tweet.
  pub body: &'a str,

  /// The content warning, if one was found before the robot's name.
  pub cw: Option<&'a str>,
}

impl RobotName<'_> {
  /// Converts the robot's prefix from UTF-8 to ASCII and removes all non-alphanumeric characters.
  fn ident(&self) -> String {
    let mut buf = unidecode(&self.prefix).to_lowercase();
    buf.retain(|c| c.is_ascii_alphanumeric());
    buf
  }
}

#[derive(PartialEq, Eq, Debug)]
struct ParseOut<'a, T> {
  output: T,
  remainder: &'a str,
}

impl<'a, T> ParseOut<'a, T> {
  const fn new(remainder: &'a str, output: T) -> Self {
    ParseOut { output, remainder }
  }
}

/// Attempt to parse a robots post. A post normally contains a single robot, but may sometimes
/// contain several (hence why we return a "group").
pub fn parse_group(text: &str) -> Option<ParsedGroup> {
  const MAX_GROUP_SIZE: usize = 5;

  fn body_re() -> &'static Regex {
    static BODY_RE: OnceLock<Regex> = OnceLock::new();
    // Meaning                             | Regex fragment
    // =====================================================
    // Allow . to match newlines           | (?s)
    // Word character                      |     \w
    // Zero or more of any character       |       .*
    // End of string                       |         $
    BODY_RE.get_or_init(|| Regex::new(r"(?s)\w.*$").unwrap())
  }

  let s = text.trim_start();

  let ParseOut {
    remainder: s,
    output: cw,
  } = parse_cw(s);

  let ParseOut {
    remainder: s,
    output: n_range,
  } = parse_numbers(s)?;

  let min_number = *n_range.start();
  let num_numbers = (*n_range.end() - *n_range.start())
    .checked_add(1)
    .and_then(|n| usize::try_from(n).ok())
    .map(|n| n.min(MAX_GROUP_SIZE))
    .unwrap_or(MAX_GROUP_SIZE);

  let ParseOut {
    remainder: s,
    output: (names, partial_names),
  } = parse_names(s, num_numbers.min(MAX_GROUP_SIZE))?;

  let body = body_re().find(s).map(|m| m.as_str()).unwrap_or("");

  let robots = names
    .into_iter()
    .enumerate()
    .map(|(i, name)| Robot {
      number: min_number + (i as i32),
      name: if partial_names {
        RobotName {
          plural: None,
          ..name
        }
      } else {
        name
      },
    })
    .collect::<Vec<Robot>>();

  Some(ParsedGroup { robots, body, cw })
}

fn parse_cw(s: &str) -> ParseOut<Option<&str>> {
  fn cw_re() -> &'static Regex {
    static CW_RE: OnceLock<Regex> = OnceLock::new();
    CW_RE.get_or_init(|| Regex::new(r"^\s*[\[\(](.+:)?\W*(\S[^\]\)]+)[\]\)]").unwrap())
  }

  let captures = match cw_re().captures(s) {
    Some(cs) => cs,
    None => return ParseOut::new(s, None),
  };

  let match_end = captures.get(0).unwrap().end();
  let warning_type = captures.get(2).unwrap().as_str().trim();

  ParseOut::new(s[match_end..].trim_start(), Some(warning_type))
}

/// Parse the prefix of the post indicating the numbers of the robots.
///
/// There's normally just one number, but we occasionally get posts with multiple robots, in which
/// case there'll be multiple numbers in an inconsistent format we have to try to infer a numerical
/// range from.
fn parse_numbers(s: &str) -> Option<ParseOut<RangeInclusive<i32>>> {
  // Numbers prefix always ends with a (lonely) closing parenthesis.
  let (s, rem) = s.split_once(')')?;

  let s = s.trim();
  let rem = rem.trim_start();

  let mut ns = Vec::<i32>::new();

  let mut buf = String::new();
  let mut neg = false;
  let mut neg_enabled = true;
  let mut found_digit = false;

  fn parse_number(buf: &str, neg: bool) -> Option<i32> {
    buf
      .parse::<i32>()
      .ok()
      .map(|n| n * if neg { -1 } else { 1 })
  }

  for c in s.chars() {
    if c.is_ascii_digit() {
      found_digit = true;
      // Once we've found our first ascii digit, we stop interpreting dashes as negative numbers
      // and start interpreting them as delimiters separating numbers.
      neg_enabled = false;
      buf.push(c);
    } else {
      // If we reach a non-digit character, consider this to be the end of the current number and
      // move on to the next one, if any.
      if !buf.is_empty() {
        ns.push(parse_number(&buf, neg)?);
        buf.clear();
      }
      if c == '-' {
        if neg_enabled {
          neg = true;
        }
      } else {
        neg = false;
        neg_enabled = true;
        // If we hit a character that was neither a digit not a minus before the first digit,
        // we're probably not parsing a valid robot post, so return None.
        if !found_digit {
          return None;
        }
      }
    }
  }

  if !buf.is_empty() {
    ns.push(parse_number(&buf, neg)?);
  }

  Some(ParseOut::new(rem, numbers_range(&ns)?))
}

/// Attempt to infer a numerical range from a sequence of numbers we got from human-written text.
fn numbers_range(ns: &[i32]) -> Option<RangeInclusive<i32>> {
  let (&first, rest) = ns.split_first()?;

  if ns.len() == 1 {
    return Some(first..=first);
  }

  let (mut min_n, mut max_n) = (first, first);

  for &n in rest {
    let n = if n > 0 && n < first.abs() {
      // If the number was less than the first number, assume it's some kind of abbreviation
      // e.g. for salt- and pepperbots, "558/9" is used to mean "558 and 559".
      let mut major = first;
      let mut dps = 0;
      let mut x = n;
      while x > 0 {
        major /= 10;
        dps += 1;
        x /= 10;
      }
      for _ in 0..dps {
        major *= 10;
      }
      major + (n * first.signum())
    } else {
      n
    };

    if n < min_n {
      min_n = n;
    } else if n > max_n {
      max_n = n;
    }
  }

  Some(min_n..=max_n)
}

fn parse_names(s: &str, target_n: usize) -> Option<ParseOut<(Vec<RobotName>, bool)>> {
  fn bot_re() -> &'static Regex {
    static BOT_RE: OnceLock<Regex> = OnceLock::new();
    // Meaning                            | Regex fragment
    // =======================================================================================
    // First matching group               | (   )
    // One or more non-whitespace         |  \S+
    // Second matching group              |      (                            )
    // Uppercase or lowercase B           |       [Bb]
    // 0 or more non-word, non-whitespace |           [^\w\s]*
    // Uppercase or lowercase O           |                   [Oo]
    // 0 or more non-word, non-whitespace |                       [^\w\s]*
    // Uppercase or lowercase T           |                               [Tt]
    // Third matching group, optional     |                                    (            )?
    // 0 or more non-word, non-whitespace |                                     [^\w\s]*
    // Uppercase or lowercase S           |                                             [Ss]
    BOT_RE
      .get_or_init(|| Regex::new(r"(\S+)([Bb][^\w\s]*[Oo][^\w\s]*[Tt])([^\w\s]*[Ss])?").unwrap())
  }

  fn partial_bot_re() -> &'static Regex {
    static PARTIAL_BOT_RE: OnceLock<Regex> = OnceLock::new();
    // Meaning                                    | Regex fragment
    // =======================================================================================
    // Beginning of the string                    | ^
    // First matching group                       |  (      )
    // 2 or more word characters                  |   \w{2,}
    // Second matching group, optional            |          ( )?
    // Hyphen character literal                   |           -
    PARTIAL_BOT_RE.get_or_init(|| Regex::new(r"^(\w{2,})(-)?").unwrap())
  }

  let mut names = Vec::<RobotName>::new();
  let mut first_match = true;
  let mut matches_start = 0;
  let mut matches_end = 0;

  for caps in bot_re().captures_iter(s) {
    if names.len() == target_n {
      break;
    }

    names.push(RobotName {
      prefix: Cow::Borrowed(caps.get(1).unwrap().as_str()),
      suffix: Cow::Borrowed(caps.get(2).unwrap().as_str()),
      plural: caps.get(3).map(|m| Cow::Borrowed(m.as_str())),
    });

    let full_match = caps.get(0).unwrap();
    if first_match {
      first_match = false;
      matches_start = full_match.start();
    }
    matches_end = full_match.end();
  }

  if names.is_empty() {
    return None;
  }

  // If the post's numbers prefix contained more numbers than we found robot names, then assume
  // that there's shorthand being used for some of the robot names. We refer to these as "partial
  // names"; they do not end with a "bot" suffix.
  //
  // For example: https://twitter.com/smolrobots/status/1017732640970067969
  // starts with "558/9)" which we expand to `[558, 559]`, but there's only a single full robot
  // name "Pepperbots". We have to match partial names to find "Salt-".
  let use_partial_names = names.len() < target_n && matches_start > 0;

  if use_partial_names {
    // How many more robot names are we looking for?
    let diff = target_n - names.len();
    let s = &s[..matches_start];

    let first_suffix = names[0].suffix.clone();
    let first_plural = names[0].plural.clone();

    let partial_names = s
      .split_whitespace()
      .filter(|&w| w.to_lowercase() != "and")
      // Apply the partial name regex to each word.
      .map(|w| partial_bot_re().captures(w))
      .flatten()
      .filter(|m| m[1].chars().any(|c| !c.is_ascii_digit()))
      .map(|m| RobotName {
        prefix: Cow::Borrowed(m.get(1).unwrap().as_str()),
        // Fill in the missing "bot" suffix with the suffix of one of the full robot names we
        // found. We choose the first one arbitrarily.
        suffix: first_suffix.clone(),
        plural: first_plural.clone(),
      });

    for (i, name) in partial_names.take(diff).enumerate() {
      names.insert(i, name);
    }
  }

  Some(ParseOut::new(&s[matches_end..], (names, use_partial_names)))
}

#[cfg(test)]
mod tests {
  use super::{ParseOut, ParsedGroup, RobotName};

  #[test]
  fn test_parse_numbers() {
    use super::parse_numbers;

    assert_eq!(parse_numbers("123)"), Some(ParseOut::new("", 123..=123)));
    assert_eq!(
      parse_numbers("123) Teabot"),
      Some(ParseOut::new("Teabot", 123..=123))
    );
    assert_eq!(
      parse_numbers("  123  )  Teabot  "),
      Some(ParseOut::new("Teabot  ", 123..=123))
    );
    assert_eq!(parse_numbers("-1)"), Some(ParseOut::new("", -1..=-1)));
    assert_eq!(parse_numbers("1, 2, 3)"), Some(ParseOut::new("", 1..=3)));
    assert_eq!(
      parse_numbers("123-124)"),
      Some(ParseOut::new("", 123..=124))
    );
    assert_eq!(
      parse_numbers("123 - 124)"),
      Some(ParseOut::new("", 123..=124))
    );
    assert_eq!(
      parse_numbers("123 & 4)"),
      Some(ParseOut::new("", 123..=124))
    );
    assert_eq!(
      parse_numbers("123 & 24)"),
      Some(ParseOut::new("", 123..=124))
    );
    assert_eq!(
      parse_numbers("124 & 3)"),
      Some(ParseOut::new("", 123..=124))
    );
    assert_eq!(parse_numbers("8, 7)"), Some(ParseOut::new("", 7..=8)));
    assert_eq!(
      parse_numbers("124-123)"),
      Some(ParseOut::new("", 123..=124))
    );
    assert_eq!(
      parse_numbers("1024 - 1048)"),
      Some(ParseOut::new("", 1024..=1048))
    );
    assert_eq!(
      parse_numbers("1024, 5 & 6)"),
      Some(ParseOut::new("", 1024..=1026))
    );
    assert_eq!(
      parse_numbers("1039, 8 & 40)"),
      Some(ParseOut::new("", 1038..=1040))
    );
    assert_eq!(parse_numbers("123"), None);
    assert_eq!(parse_numbers("Foo baa"), None);
    assert_eq!(
      parse_numbers("2147483646)"),
      Some(ParseOut::new("", 2147483646..=2147483646))
    );
    assert_eq!(
      parse_numbers("2147483647)"),
      Some(ParseOut::new("", 2147483647..=2147483647))
    );
    assert_eq!(parse_numbers("2147483648)"), None);
    assert_eq!(
      parse_numbers("2147483646 - 2147483647)"),
      Some(ParseOut::new("", 2147483646..=2147483647))
    );
    assert_eq!(parse_numbers("2147483646 - 2147483648)"), None);
    assert_eq!(parse_numbers("Hello)"), None);
    assert_eq!(parse_numbers("@foo 123)"), None);
    assert_eq!(parse_numbers("@foo123)"), None);
  }

  #[test]
  fn test_parse_names() {
    use super::parse_names;

    assert_eq!(
      parse_names("Teabot. Brings you tea", 1),
      Some(ParseOut::new(
        ". Brings you tea",
        (
          vec![RobotName {
            prefix: "Tea".into(),
            suffix: "bot".into(),
            plural: None
          }],
          false
        )
      ))
    );

    assert_eq!(
      parse_names("Mischiefbots. Oh no!!", 1),
      Some(ParseOut::new(
        ". Oh no!!",
        (
          vec![RobotName {
            prefix: "Mischief".into(),
            suffix: "bot".into(),
            plural: Some("s".into())
          }],
          false
        )
      ))
    );

    assert_eq!(
      parse_names("R.O.B.O.T.S.", 1),
      Some(ParseOut::new(
        ".",
        (
          vec![RobotName {
            prefix: "R.O.".into(),
            suffix: "B.O.T".into(),
            plural: Some(".S".into())
          }],
          false
        )
      ))
    );

    assert_eq!(
      parse_names("Saltbot and pepperbot.", 1),
      Some(ParseOut::new(
        " and pepperbot.",
        (
          vec![RobotName {
            prefix: "Salt".into(),
            suffix: "bot".into(),
            plural: None
          }],
          false
        )
      ))
    );

    assert_eq!(
      parse_names("Saltbot and pepperbot.", 2),
      Some(ParseOut::new(
        ".",
        (
          vec![
            RobotName {
              prefix: "Salt".into(),
              suffix: "bot".into(),
              plural: None
            },
            RobotName {
              prefix: "pepper".into(),
              suffix: "bot".into(),
              plural: None
            }
          ],
          false
        )
      ))
    );

    assert_eq!(
      parse_names("Saltbot and pepperbot.", 3),
      Some(ParseOut::new(
        ".",
        (
          vec![
            RobotName {
              prefix: "Salt".into(),
              suffix: "bot".into(),
              plural: None
            },
            RobotName {
              prefix: "pepper".into(),
              suffix: "bot".into(),
              plural: None
            }
          ],
          false
        )
      ))
    );

    assert_eq!(
      parse_names("Salt- and pepperbots.", 2),
      Some(ParseOut::new(
        ".",
        (
          vec![
            RobotName {
              prefix: "Salt".into(),
              suffix: "bot".into(),
              plural: Some("s".into())
            },
            RobotName {
              prefix: "pepper".into(),
              suffix: "bot".into(),
              plural: Some("s".into())
            }
          ],
          true
        )
      ))
    );
  }

  #[test]
  fn test_parse_group() {
    use super::{parse_group, Robot};

    assert_eq!(
            parse_group("1207) Transrightsbot. Is just here to let all its trans pals know that they are valid and they are loved! \u{1f3f3}\u{fe0f}\u{200d}\u{26a7}\u{fe0f}\u{2764}\u{fe0f}\u{1f916}"),
            Some(ParsedGroup { robots: vec![Robot { number: 1207, name: RobotName { prefix: "Transrights".into(), suffix: "bot".into(), plural: None } }], body: "Is just here to let all its trans pals know that they are valid and they are loved! \u{1f3f3}\u{fe0f}\u{200d}\u{26a7}\u{fe0f}\u{2764}\u{fe0f}\u{1f916}", cw: None })
        );

    assert_eq!(
      parse_group("558/9) Salt- and Pepperbots. Bring you salt and pepper."),
      Some(ParsedGroup {
        robots: vec![
          Robot {
            number: 558,
            name: RobotName {
              prefix: "Salt".into(),
              suffix: "bot".into(),
              plural: None
            }
          },
          Robot {
            number: 559,
            name: RobotName {
              prefix: "Pepper".into(),
              suffix: "bot".into(),
              plural: None
            }
          }
        ],
        body: "Bring you salt and pepper.",
        cw: None
      })
    );

    assert_eq!(
            parse_group("690 - 692) Marybot, Josephbot and Donkeybot. For complicated tax reasons, Marybot and Josephbot are forced to temporarily relocate to Bethlehem, just as Marybot recieves a mysterious package from Gabrielbot on behalf of Godbot Labs."),
            Some(ParsedGroup { robots: vec![Robot { number: 690, name: RobotName { prefix: "Mary".into(), suffix: "bot".into(), plural: None } }, Robot { number: 691, name: RobotName { prefix: "Joseph".into(), suffix: "bot".into(), plural: None } }, Robot { number: 692, name: RobotName { prefix: "Donkey".into(), suffix: "bot".into(), plural: None } }], body: "For complicated tax reasons, Marybot and Josephbot are forced to temporarily relocate to Bethlehem, just as Marybot recieves a mysterious package from Gabrielbot on behalf of Godbot Labs.", cw: None })
        );

    assert_eq!(
            parse_group("[CN: sexual assault] 651) Believeherbot. Reminds you to believe the testimony of women survivors of sexual assault; reminds you to look at the gendered power structures in place before you dismiss them as unreliable; reminds you that this is the fucking turning point."),
            Some(ParsedGroup { robots: vec![Robot { number: 651, name: RobotName { prefix: "Believeher".into(), suffix: "bot".into(), plural: None } }], body: "Reminds you to believe the testimony of women survivors of sexual assault; reminds you to look at the gendered power structures in place before you dismiss them as unreliable; reminds you that this is the fucking turning point.", cw: Some("sexual assault") })
        );
  }
}
