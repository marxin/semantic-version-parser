//! # semantic-version-parser
//!
//! A library providing parsing capability for various version string formats (e.g. `2.3.0-alpha.1`).
//! The parsed `SemVer` type provides capability for displaying the version in [PHP composer format](https://getcomposer.org/doc/04-schema.md#version)
//! and one can use the provided methods for incrementing of *major*, *minor* and the *patch* parts of the version.
//!
//! # Example
//!
//! ```
//! use std::str::FromStr;
//! use semantic_version_parser::SemVer;
//!
//! let semver = SemVer::from_str("1.2.3.beta.5").unwrap();
//! assert_eq!("2.2.3-beta5", semver.increment_major().to_string());
//! ```
//!
//! The library also contains a PHP composer checker `ComposerChecker` one can use to verify if the output string version
//! is really supported.
//!
//! ```
//! use semantic_version_parser::ComposerChecker;
//!
//! let checker = ComposerChecker::new();
//! assert!(checker.is_valid("2.2.3-beta5"));
//! assert!(!checker.is_valid("2.2.3.beta.5"));
//! assert!(!checker.is_valid("1.0.0.2"));
//! ```

use chrono::prelude::*;
use itertools::{self, Itertools};
use regex::Regex;
use std::fmt;
use std::num::ParseIntError;
use std::ops::Add;
use std::str::FromStr;
use strum_macros::EnumString;

pub struct ComposerChecker {
    regex: Regex,
}

impl Default for ComposerChecker {
    fn default() -> Self {
        Self::new()
    }
}

// FIXME: The regex should reject the following suffix: `dev123`
impl ComposerChecker {
    pub fn new() -> Self {
        Self {
            regex: Regex::new(r"^v?(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)(-(?P<suffix>dev|d|patch|p|alpha|a|beta|b|RC)(?P<suffix_version>\d+)?)?$").unwrap()
        }
    }

    pub fn is_valid(&self, version: &str) -> bool {
        self.regex.is_match(version)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, EnumString, strum_macros::Display)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
enum SemVerPrefix {
    V,
}

#[derive(Debug, PartialEq, Eq, Clone, EnumString, strum_macros::Display, Default)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
enum SemVerSuffix {
    Dev,
    Patch,
    #[default]
    P,
    Alpha,
    A,
    Beta,
    B,
    #[strum(serialize = "RC")]
    RC,
}

// FIXME: technically "dev44" should not be supported

#[derive(Debug, PartialEq, Eq, Clone)]
struct SemVerPair {
    suffix: SemVerSuffix,
    version: Option<i32>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct ZeroPaddedInt {
    value: u32,
    width: usize,
}

impl ZeroPaddedInt {
    #[allow(dead_code)]
    fn new(value: u32, width: usize) -> Self {
        Self { value, width }
    }
}

impl From<u32> for ZeroPaddedInt {
    fn from(value: u32) -> Self {
        ZeroPaddedInt {
            value,
            width: value.to_string().chars().count(),
        }
    }
}

impl FromStr for ZeroPaddedInt {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.parse::<u32>()?;
        Ok(ZeroPaddedInt {
            value,
            width: s.chars().count(),
        })
    }
}

impl Add<u32> for ZeroPaddedInt {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        Self {
            value: self.value + rhs,
            ..self
        }
    }
}

impl fmt::Display for ZeroPaddedInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:0width$}", self.value, width = self.width)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SemVer {
    prefix: Option<SemVerPrefix>,
    major: ZeroPaddedInt,
    minor: ZeroPaddedInt,
    patch: ZeroPaddedInt,
    suffix: Option<SemVerPair>,
}

impl SemVer {
    pub fn increment_major(self) -> Self {
        Self {
            major: self.major + 1,
            ..self
        }
    }

    pub fn increment_minor(self) -> Self {
        Self {
            minor: self.minor + 1,
            ..self
        }
    }

    pub fn increment_patch(self) -> Self {
        Self {
            patch: self.patch + 1,
            ..self
        }
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}.{}.{}{}",
            self.prefix
                .as_ref()
                .map_or_else(|| "".to_string(), |p| p.to_string()),
            self.major,
            self.minor,
            self.patch,
            self.suffix.as_ref().map_or_else(
                || "".to_string(),
                |suffix| format!(
                    "-{}{}",
                    suffix.suffix,
                    suffix
                        .version
                        .map_or_else(|| "".to_string(), |v| v.to_string())
                )
            )
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseSemVerError;

impl FromStr for SemVer {
    type Err = ParseSemVerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s
            .split(&['-', '_', '.'])
            .flat_map(split_alpha_and_number)
            .map(|t| t.to_lowercase().to_string())
            .collect_vec();
        if parts.is_empty() {
            return Err(ParseSemVerError);
        }

        // 1) Parse the prefix component, if any.
        let prefix = SemVerPrefix::from_str(&parts[0]).ok();

        // support: release-2022-02-09
        if prefix.is_some() || parts[0] == "release" {
            parts.remove(0);
        }
        // support month name as the second component
        if let Ok(month) = parts[1].parse::<Month>() {
            parts[1] = month.number_from_month().to_string();
        }

        match parts.len() {
            ..=1 => {
                return Err(ParseSemVerError);
            }
            2 => {
                parts.push("0".to_string());
            }
            _ => {}
        }

        // 2) Parse the suffix component, if any present.
        let mut suffix_part = None;
        if parts.len() >= 4 {
            suffix_part = SemVerSuffix::from_str(&parts[3]).ok();
            if suffix_part.is_some() {
                parts.remove(3);
            } else {
                // support: 2023-11-29-v1
                if parts[3] == "v" {
                    parts.remove(3);
                }
            }
        }

        // 3) Parse the suffix version (4 version number in the format).
        let mut suffix_version = None;
        if parts.len() >= 4 {
            suffix_version = Some(parts[3].parse::<i32>().unwrap());
            parts.remove(3);

            // Make a default suffix name "P" if the is not any.
            if suffix_part.is_none() {
                suffix_part = Some(SemVerSuffix::default());
            }
        }
        let suffix = suffix_part.map(|sp| SemVerPair {
            suffix: sp,
            version: suffix_version,
        });

        // FIXME: if any part is not a number, the following code will panic
        let integer_parts = parts
            .iter()
            .take(3)
            .map(|p| ZeroPaddedInt::from_str(p).unwrap())
            .collect_vec();

        Ok(SemVer {
            major: integer_parts[0],
            minor: integer_parts[1],
            patch: integer_parts[2],
            prefix,
            suffix,
        })
    }
}

fn split_alpha_and_number(s: &str) -> Vec<&str> {
    let number_start = s.chars().position(|c| c.is_numeric());
    if let Some(number_start) = number_start {
        if number_start > 0 {
            return vec![&s[0..number_start], &s[number_start..]];
        }
    }

    vec![s]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composer_valid_strings() {
        let checker = ComposerChecker::new();
        assert!(checker.is_valid("1.2.3"));
        assert!(checker.is_valid("v1.2.3"));
        assert!(checker.is_valid("v1.2.3"));

        let official_examples = vec![
            "1.0.0",
            "1.0.2",
            "1.1.0",
            "0.2.5",
            "1.0.0-dev",
            "1.0.0-alpha3",
            "1.0.0-beta2",
            "1.0.0-RC5",
            "v2.0.4-p1",
        ];
        for example in official_examples {
            assert!(checker.is_valid(example));
        }

        assert!(!checker.is_valid("release1.2.3"));
        assert!(!checker.is_valid("1.2.3.4"));
        assert!(!checker.is_valid("1.2.3-dev.1"));
        assert!(!checker.is_valid("1.2.3-foobar"));
    }

    #[test]
    fn semantic_version_prefix_parsing() {
        assert_eq!(SemVerPrefix::from_str("V"), Ok(SemVerPrefix::V));
        assert_eq!(SemVerPrefix::from_str("v"), Ok(SemVerPrefix::V));
        assert!(SemVerPrefix::from_str("RC").is_err());
        assert_eq!(SemVerPrefix::V.to_string(), "v".to_string());
    }

    #[test]
    fn semantic_version_suffix_parsing() {
        assert_eq!(SemVerSuffix::from_str("BETA"), Ok(SemVerSuffix::Beta));
        assert_eq!(SemVerSuffix::from_str("b"), Ok(SemVerSuffix::B));
        assert_eq!(SemVerSuffix::from_str("RC"), Ok(SemVerSuffix::RC));
        assert_eq!(SemVerSuffix::B.to_string(), "b");
        assert_eq!(SemVerSuffix::RC.to_string(), "RC");
    }

    #[test]
    fn alpha_and_number_parts() {
        assert_eq!(split_alpha_and_number("rc123"), vec!["rc", "123"]);
        assert_eq!(split_alpha_and_number("test"), vec!["test"]);
        assert_eq!(split_alpha_and_number("123"), vec!["123"]);
        assert_eq!(split_alpha_and_number("rc1beta"), vec!["rc", "1beta"]);
    }

    #[test]
    fn chrono_month_parsing() {
        assert_eq!("Feb".parse::<Month>(), Ok(Month::February));
        assert_eq!("Nov".parse::<Month>(), Ok(Month::November));
        assert_eq!("nov".parse::<Month>(), Ok(Month::November));
    }

    #[test]
    fn parse_valid_semantic_versions() {
        assert_eq!(
            SemVer::from_str("v1.2.3"),
            Ok(SemVer {
                prefix: Some(SemVerPrefix::V),
                major: ZeroPaddedInt::from(1),
                minor: ZeroPaddedInt::from(2),
                patch: ZeroPaddedInt::from(3),
                suffix: None
            })
        );
        assert_eq!(
            SemVer::from_str("2.1.0-beta1"),
            Ok(SemVer {
                prefix: None,
                major: ZeroPaddedInt::from(2),
                minor: ZeroPaddedInt::from(1),
                patch: ZeroPaddedInt::from(0),
                suffix: Some(SemVerPair {
                    suffix: SemVerSuffix::Beta,
                    version: Some(1)
                })
            })
        );
        assert_eq!(
            SemVer::from_str("release-2022-02-09"),
            Ok(SemVer {
                prefix: None,
                major: ZeroPaddedInt::from(2022),
                minor: ZeroPaddedInt::new(2, 2),
                patch: ZeroPaddedInt::new(9, 2),
                suffix: None
            })
        );
        assert_eq!(
            SemVer::from_str("09-28-2023.1"),
            Ok(SemVer {
                prefix: None,
                major: ZeroPaddedInt::new(9, 2),
                minor: ZeroPaddedInt::from(28),
                patch: ZeroPaddedInt::from(2023),
                suffix: Some(SemVerPair {
                    suffix: SemVerSuffix::P,
                    version: Some(1)
                })
            })
        );
        assert_eq!(
            SemVer::from_str("2023-11-29-v1"),
            Ok(SemVer {
                prefix: None,
                major: ZeroPaddedInt::from(2023),
                minor: ZeroPaddedInt::from(11),
                patch: ZeroPaddedInt::from(29),
                suffix: Some(SemVerPair {
                    suffix: SemVerSuffix::P,
                    version: Some(1)
                })
            })
        );
        assert_eq!(
            SemVer::from_str("1.0.0-alpha.0"),
            Ok(SemVer {
                prefix: None,
                major: ZeroPaddedInt::from(1),
                minor: ZeroPaddedInt::from(0),
                patch: ZeroPaddedInt::from(0),
                suffix: Some(SemVerPair {
                    suffix: SemVerSuffix::Alpha,
                    version: Some(0)
                })
            })
        );

        assert_eq!(
            SemVer::from_str("2023-Nov-27-v1"),
            Ok(SemVer {
                prefix: None,
                major: ZeroPaddedInt::from(2023),
                minor: ZeroPaddedInt::from(11),
                patch: ZeroPaddedInt::from(27),
                suffix: Some(SemVerPair {
                    suffix: SemVerSuffix::P,
                    version: Some(1)
                })
            })
        );
    }

    #[test]
    #[should_panic]
    fn parse_invalid_semantic_versions() {
        assert!(SemVer::from_str("foo.bar.baz").is_err());
    }

    #[test]
    fn output_format_with_month_name() {
        let semver = SemVer::from_str("v2023-Nov-27-v1").unwrap();
        assert_eq!(semver.to_string(), "v2023.11.27-p1");
    }

    #[test]
    fn parse_all_provided_versions() {
        let checker = ComposerChecker::new();
        let versions = std::fs::read_to_string("test-input/versions.txt").unwrap();
        for version in versions.split(',').map(|part| part.trim()) {
            if version == "list" {
                // ignored input value
                continue;
            }

            let semver = SemVer::from_str(dbg!(version));
            assert!(checker.is_valid(&dbg!(semver).unwrap().to_string()));
        }
    }

    #[test]
    fn increment_version() {
        let semver = SemVer::from_str("v2023-Nov-0027-v1").unwrap();
        assert_eq!(
            semver.to_owned().increment_major().to_string(),
            "v2024.11.0027-p1"
        );
        assert_eq!(
            semver.to_owned().increment_minor().to_string(),
            "v2023.12.0027-p1"
        );
        assert_eq!(
            semver.to_owned().increment_patch().to_string(),
            "v2023.11.0028-p1"
        );
    }
}
