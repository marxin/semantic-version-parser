use chrono::prelude::*;
use itertools::{self, Itertools};
use std::fmt;
use std::str::FromStr;
use strum_macros::EnumString;

#[derive(Debug, PartialEq, Eq, Clone, EnumString, strum_macros::Display)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
enum SemVerPrefix {
    V,
}

#[derive(Debug, PartialEq, Eq, Clone, EnumString, strum_macros::Display)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
enum SemVerSuffix {
    Dev,
    Patch,
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SemVer {
    prefix: Option<SemVerPrefix>,
    major: i32,
    minor: i32,
    patch: i32,
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

            // Make a default suffix name "P" if the is not any.
            if suffix_part.is_none() {
                suffix_part = Some(SemVerSuffix::P);
            }
        }
        let suffix = suffix_part.map(|sp| SemVerPair {
            suffix: sp,
            version: suffix_version,
        });

        // TODO
        let integer_parts = parts
            .iter()
            .take(3)
            .map(|p| p.parse::<i32>().unwrap())
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
                major: 1,
                minor: 2,
                patch: 3,
                suffix: None
            })
        );
        assert_eq!(
            SemVer::from_str("2.1.0-beta1"),
            Ok(SemVer {
                prefix: None,
                major: 2,
                minor: 1,
                patch: 0,
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
                major: 2022,
                minor: 2,
                patch: 9,
                suffix: None
            })
        );
        assert_eq!(
            SemVer::from_str("09-28-2023.1"),
            Ok(SemVer {
                prefix: None,
                major: 9,
                minor: 28,
                patch: 2023,
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
                major: 2023,
                minor: 11,
                patch: 29,
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
                major: 1,
                minor: 0,
                patch: 0,
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
                major: 2023,
                minor: 11,
                patch: 27,
                suffix: Some(SemVerPair {
                    suffix: SemVerSuffix::P,
                    version: Some(1)
                })
            })
        );
    }

    #[test]
    fn output_format() {
        let semver = SemVer::from_str("v2023-Nov-27-v1").unwrap();
        assert_eq!(semver.to_string(), "v2023.11.27-p1");
    }

    #[test]
    fn parse_all_provided_versions() {
        let versions = std::fs::read_to_string("test-input/versions.txt").unwrap();
        for version in versions.split(',').map(|part| part.trim()) {
            let semver = SemVer::from_str(dbg!(version));
            assert!(dbg!(semver).is_ok());
        }
    }

    #[test]
    fn increment_version() {
        let semver = SemVer::from_str("v2023-Nov-27-v1").unwrap();
        assert_eq!(
            semver.to_owned().increment_major().to_string(),
            "v2024.11.27-p1"
        );
        assert_eq!(
            semver.to_owned().increment_minor().to_string(),
            "v2023.12.27-p1"
        );
        assert_eq!(
            semver.to_owned().increment_patch().to_string(),
            "v2023.11.28-p1"
        );
    }
}
