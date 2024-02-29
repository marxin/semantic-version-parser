use core::num;
use std::str::FromStr;
use itertools::{self, Itertools};
use strum_macros::EnumString;

#[derive(Debug, PartialEq, Eq, EnumString, strum_macros::Display)]
#[strum(serialize_all="lowercase", ascii_case_insensitive)]
enum SemanticVersionPrefix {
    V
}

#[derive(Debug, PartialEq, Eq, EnumString, strum_macros::Display)]
#[strum(serialize_all="lowercase", ascii_case_insensitive)]
enum SemanticVersionSuffix {
    Dev,
    Patch,
    P,
    Alpha,
    A,
    Beta,
    B,
    #[strum(serialize="RC")]
    RC
}

// FIXME: technically "dev44" should not be supported

#[derive(Debug, PartialEq, Eq)]
struct SemanticVersionPair {
    suffix: SemanticVersionSuffix,
    version: Option<i32>,
}

#[derive(Debug, PartialEq, Eq)]
struct SemanticVersion {
    prefix: Option<SemanticVersionPrefix>,
    major: i32,
    minor: i32,
    patch: i32,
    suffix: Option<SemanticVersionPair>,
}

#[derive(Debug, PartialEq, Eq)]
struct ParseSemanticVersionError;

impl FromStr for SemanticVersion {
    type Err = ParseSemanticVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(&['-', '_', '.']).map(split_alpha_and_number).flatten().collect_vec();
        if parts.is_empty() {
            return Err(ParseSemanticVersionError)
        }

        // 1) Parse the prefix component, if any.

        let prefix = SemanticVersionPrefix::from_str(parts[0]).ok();

        // support: release-2022-02-09
        if prefix.is_some() || parts[0] == "release" {
            parts.remove(0);
        }
        
        if parts.len() < 3 {
            return Err(ParseSemanticVersionError)
        }


        // 2) Parse the suffix component, if any present.
        let mut suffix_part = None;
        if parts.len() >= 4 {
            suffix_part = SemanticVersionSuffix::from_str(parts[3]).ok();
            if suffix_part.is_some() {
                parts.remove(3);
            }

            // support: 2023-11-29-v1
            if parts[3] == "v" {
                parts.remove(3);
            }
        }

        // 3) Parse the suffix version (4 version number in the format).
        let mut suffix_version = None;
        if parts.len() >= 4 {
            suffix_version = Some(parts[3].parse::<i32>().unwrap());

            // Make a default suffix name "P" if the is not any.
            if suffix_part.is_none() {
                suffix_part = Some(SemanticVersionSuffix::P);
            }
        }
        let suffix = {
            if suffix_part.is_some() {
                Some(SemanticVersionPair { suffix: suffix_part.unwrap(), version: suffix_version })
            } else {
                None
            }
        };

        println!("{parts:?}, prefix={prefix:?}, suffix={suffix:?}");

        // TODO
        let integer_parts = parts.iter().take(3).map(|p| p.parse::<i32>().unwrap()).collect_vec();

        Ok (SemanticVersion { major: integer_parts[0], minor: integer_parts[1], patch: integer_parts[2], prefix, suffix })
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
        assert_eq!(SemanticVersionPrefix::from_str("V"), Ok(SemanticVersionPrefix::V));
        assert_eq!(SemanticVersionPrefix::from_str("v"), Ok(SemanticVersionPrefix::V));
        assert!(SemanticVersionPrefix::from_str("RC").is_err());
        assert_eq!(SemanticVersionPrefix::V.to_string(), "v".to_string());
    }

    #[test]
    fn semantic_version_suffix_parsing() {
        assert_eq!(SemanticVersionSuffix::from_str("BETA"), Ok(SemanticVersionSuffix::Beta));
        assert_eq!(SemanticVersionSuffix::from_str("b"), Ok(SemanticVersionSuffix::B));
        assert_eq!(SemanticVersionSuffix::from_str("RC"), Ok(SemanticVersionSuffix::RC));
        assert_eq!(SemanticVersionSuffix::B.to_string(), "b");
        assert_eq!(SemanticVersionSuffix::RC.to_string(), "RC");
    }

    #[test]
    fn alpha_and_number_parts() {
        assert_eq!(split_alpha_and_number("rc123"), vec!["rc", "123"]);
        assert_eq!(split_alpha_and_number("test"), vec!["test"]);
        assert_eq!(split_alpha_and_number("123"), vec!["123"]);
        assert_eq!(split_alpha_and_number("rc1beta"), vec!["rc", "1beta"]);
    }

    #[test]
    fn parse_valid_semantic_versions() {
        assert_eq!(SemanticVersion::from_str("v1.2.3"), Ok(SemanticVersion { prefix: Some(SemanticVersionPrefix::V), major: 1, minor: 2, patch: 3, suffix: None }));
        assert_eq!(SemanticVersion::from_str("2.1.0-beta1"), Ok(SemanticVersion { prefix: None, major: 2, minor: 1, patch: 0, suffix: Some(SemanticVersionPair { suffix: SemanticVersionSuffix::Beta, version: Some(1) }) }));
        assert_eq!(SemanticVersion::from_str("release-2022-02-09"), Ok(SemanticVersion { prefix: None, major: 2022, minor: 2, patch: 9, suffix: None }));
        assert_eq!(SemanticVersion::from_str("09-28-2023.1"), Ok(SemanticVersion { prefix: None, major: 9, minor: 28, patch: 2023, suffix: Some(SemanticVersionPair { suffix: SemanticVersionSuffix::P, version: Some(1) }) }));
        assert_eq!(SemanticVersion::from_str("2023-11-29-v1"), Ok(SemanticVersion { prefix: None, major: 2023, minor: 11, patch: 29, suffix: Some(SemanticVersionPair { suffix: SemanticVersionSuffix::P, version: Some(1) }) }));
        assert_eq!(SemanticVersion::from_str("1.0.0-alpha.0"), Ok(SemanticVersion { prefix: None, major: 1, minor: 0, patch: 0, suffix: Some(SemanticVersionPair { suffix: SemanticVersionSuffix::Alpha, version: Some(0) }) }));
    }
}
