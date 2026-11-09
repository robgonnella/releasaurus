use chrono::{
    DateTime, TimeZone, Utc,
    format::{Item, Pad, Parsed, StrftimeItems},
};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

use crate::result::ReleasaurusError;

pub const DEFAULT_VERSION_TYPE: VersionType = VersionType::Semantic;

/// Ends a strftime `version_type` that numbers releases sharing a rendered
/// date (`%Y.%m.%d.%i` → `2026.10.05.1`, `2026.10.05.2`). chrono has no `%i`;
/// it is stripped before formatting or parsing and handled separately.
pub const COUNTER_SUFFIX: &str = ".%i";

/// Determines what type of versioning to use (semantic, date, etc.)
#[derive(Debug, Default, Clone, JsonSchema, PartialEq, Eq)]
pub enum VersionType {
    #[default]
    #[serde(rename = "major.minor.patch")]
    Semantic,
    #[serde(rename = "major.minor.patch+timestamp.sha")]
    SemanticWithBuild,
    #[serde(rename = "year.month.day")]
    SemanticDate,
    #[serde(rename = "year.month.day+hour.minute.second")]
    SemanticDateWithTime,
    #[serde(rename = "year.month.day+hour.minute.second.micro")]
    SemanticDateWithTimeMicro,
    #[serde(untagged)]
    Strftime(#[schemars(pattern("%"))] String),
}

impl FromStr for VersionType {
    type Err = ReleasaurusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let version_type = match s {
            "major.minor.patch" => Self::Semantic,
            "major.minor.patch+timestamp.sha" => Self::SemanticWithBuild,
            "year.month.day" => Self::SemanticDate,
            "year.month.day+hour.minute.second" => Self::SemanticDateWithTime,
            "year.month.day+hour.minute.second.micro" => {
                Self::SemanticDateWithTimeMicro
            }
            fmt if fmt.contains('%') => {
                validate_strftime(fmt)?;
                Self::Strftime(fmt.to_string())
            }
            _ => {
                return Err(ReleasaurusError::invalid_config(format!(
                    "unknown version_type \"{s}\": expected one of \
                     major.minor.patch, major.minor.patch+timestamp.sha, \
                     year.month.day, year.month.day+hour.minute.second, \
                     year.month.day+hour.minute.second.micro, or a strftime \
                     format string such as \"%Y.%m.%d\""
                )));
            }
        };
        Ok(version_type)
    }
}

/// Fixed so validation errors are reproducible.
fn strftime_sample() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 5, 13, 7, 9).unwrap()
}

fn validate_strftime(fmt: &str) -> Result<(), ReleasaurusError> {
    let date_fmt = fmt.strip_suffix(COUNTER_SUFFIX);
    let has_counter = date_fmt.is_some();
    let date_fmt = date_fmt.unwrap_or(fmt);

    // chrono has no %i, so a misplaced counter surfaces as an error item;
    // checking first would also reject the escaped literal %%i
    if StrftimeItems::new(date_fmt).any(|item| matches!(item, Item::Error)) {
        // %%i is an escaped literal, not a misplaced counter
        let msg = if date_fmt.replace("%%", "").contains("%i") {
            format!(
                "\"{fmt}\" places the release counter %i somewhere other \
                 than the end; it may only appear once, as the final \".%i\" \
                 segment"
            )
        } else {
            format!("\"{fmt}\" is not a valid strftime format string")
        };
        return Err(ReleasaurusError::invalid_config(msg));
    }

    // Space padding only appears for small values, which the sample
    // below may not hit, and git rejects spaces in tag names.
    if StrftimeItems::new(date_fmt)
        .any(|item| matches!(item, Item::Numeric(_, Pad::Space)))
    {
        return Err(ReleasaurusError::invalid_config(format!(
            "\"{fmt}\" pads a field with spaces (e.g. %e, %k, %_m), which \
             is not valid in a git tag name; use the zero-padded or \
             unpadded form instead (%d / %-d)"
        )));
    }

    let sample_count = has_counter.then_some(1);
    let mut sample = strftime_sample().format(date_fmt).to_string();
    if let Some(count) = sample_count {
        sample = format!("{sample}.{count}");
    }

    validate_git_tag_name(fmt, &sample)?;

    // existing tags are found by parsing their names
    let cannot_parse_back = |reason: String| {
        ReleasaurusError::invalid_config(format!(
            "\"{fmt}\" cannot be parsed back from the tag names it produces \
             (e.g. \"{sample}\"): {reason}"
        ))
    };

    let (_, parsed_count) = parse_strftime(&sample, fmt)
        .map_err(|e| cannot_parse_back(e.to_string()))?;

    if parsed_count != sample_count {
        return Err(cannot_parse_back(
            "the release counter is not recoverable".into(),
        ));
    }

    Ok(())
}

/// The `git check-ref-format` rules that hold whatever tag prefix is
/// prepended later.
fn validate_git_tag_name(
    fmt: &str,
    name: &str,
) -> Result<(), ReleasaurusError> {
    let invalid = |what: String| {
        ReleasaurusError::invalid_config(format!(
            "version_type \"{fmt}\" renders \"{name}\", which {what} and is \
             not a valid git tag name"
        ))
    };

    if let Some(c) = name.chars().find(|c| {
        c.is_ascii_control()
            || matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
    }) {
        return Err(invalid(format!("contains {c:?}")));
    }

    for seq in ["..", "@{", "//"] {
        if name.contains(seq) {
            return Err(invalid(format!("contains \"{seq}\"")));
        }
    }

    if name.ends_with('.') || name.ends_with('/') {
        return Err(invalid("ends with \".\" or \"/\"".into()));
    }

    if name
        .split('/')
        .any(|component| component.ends_with(".lock"))
    {
        return Err(invalid("has a component ending in \".lock\"".into()));
    }

    Ok(())
}

/// Fields the format omits take the first value of their period
/// (`%Y.%m` → the 1st at 00:00:00). A format ending in [`COUNTER_SUFFIX`]
/// also yields the counter; names without one, such as tags made before the
/// counter was added to the format, still parse, with no counter.
pub fn parse_strftime(
    raw: &str,
    fmt: &str,
) -> Result<(DateTime<Utc>, Option<u16>), ReleasaurusError> {
    let Some(date_fmt) = fmt.strip_suffix(COUNTER_SUFFIX) else {
        return Ok((parse_strftime_date(raw, fmt)?, None));
    };

    if let Some((date_raw, count)) = raw.rsplit_once('.')
        // integer parsing alone would accept a leading "+"
        && count.bytes().all(|b| b.is_ascii_digit())
        && let Ok(count) = count.parse::<u16>()
        && let Ok(date) = parse_strftime_date(date_raw, date_fmt)
    {
        return Ok((date, Some(count)));
    }

    Ok((parse_strftime_date(raw, date_fmt)?, None))
}

fn parse_strftime_date(
    raw: &str,
    fmt: &str,
) -> Result<DateTime<Utc>, ReleasaurusError> {
    let mut parsed = Parsed::new();
    chrono::format::parse(&mut parsed, raw, StrftimeItems::new(fmt))?;

    if parsed.to_naive_date().is_err() {
        if parsed.month().is_none() {
            parsed.set_month(1)?;
        }
        if parsed.day().is_none() {
            parsed.set_day(1)?;
        }
    }

    if parsed.to_naive_time().is_err() {
        if parsed.hour_div_12().is_none() {
            parsed.set_hour(0)?;
        }
        if parsed.minute().is_none() {
            parsed.set_minute(0)?;
        }
    }

    Ok(parsed.to_naive_datetime_with_offset(0)?.and_utc())
}

impl std::fmt::Display for VersionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Semantic => f.write_str("major.minor.patch"),
            Self::SemanticWithBuild => {
                f.write_str("major.minor.patch+timestamp.sha")
            }
            Self::SemanticDate => f.write_str("year.month.day"),
            Self::SemanticDateWithTime => {
                f.write_str("year.month.day+hour.minute.second")
            }
            Self::SemanticDateWithTimeMicro => {
                f.write_str("year.month.day+hour.minute.second.micro")
            }
            Self::Strftime(fmt) => f.write_str(fmt),
        }
    }
}

impl<'de> Deserialize<'de> for VersionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Serialize for VersionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl VersionType {
    /// Returns true for date-based version types, which derive the version
    /// from the current date/time rather than from commits. Semantic-only
    /// settings (prerelease, custom increment regexes) do not apply to
    /// these types.
    pub fn is_date_based(&self) -> bool {
        matches!(
            self,
            VersionType::SemanticDate
                | VersionType::SemanticDateWithTime
                | VersionType::SemanticDateWithTimeMicro
                | VersionType::Strftime(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_type_from_str_round_trips_named_types() {
        for version_type in [
            VersionType::Semantic,
            VersionType::SemanticWithBuild,
            VersionType::SemanticDate,
            VersionType::SemanticDateWithTime,
            VersionType::SemanticDateWithTimeMicro,
        ] {
            assert_eq!(
                VersionType::from_str(&version_type.to_string()).unwrap(),
                version_type
            );
        }
    }

    #[test]
    fn version_type_from_str_rejects_unknown_names() {
        for s in ["", "semantic", "year.month.dya", "YYYY.MM.DD"] {
            let err = VersionType::from_str(s).unwrap_err();
            assert!(matches!(err, ReleasaurusError::InvalidConfig(_)), "{s:?}");
        }
    }

    #[test]
    fn version_type_from_str_accepts_strftime_formats() {
        for fmt in [
            "%Y.%m.%d",
            "%y.%-m.%-d",
            "%Y.%m",
            "%Y",
            "%Y.%j",
            "%Y.%m.%d.%H%M",
            "%Y%m%d-%H%M%S",
            "%Y/%m",
            "%Y.%B",
            "%Y%m%d%H%M%S%f",
            "%Y.%m.%d.%H%M%S%6f",
            "%Y.%m.%d.%H%M%S%.3f",
            "%Y.%m.%d.%i",
            "%Y.%-m.%-d.%i",
            "%Y%m%d.%i",
            "%Y.%m.%%i",
            "%Y.%m.%%i.%i",
        ] {
            assert_eq!(
                VersionType::from_str(fmt).unwrap(),
                VersionType::Strftime(fmt.to_string()),
                "{fmt:?}"
            );
        }
    }

    #[test]
    fn version_type_from_str_rejects_misplaced_release_counter() {
        for fmt in [
            "%Y.%i.%m",
            "%i",
            ".%i",
            "%Y.%m.%d.%i.%i",
            "%Y%m%d%i",
            "%Y.%m.%d-%i",
        ] {
            let err = VersionType::from_str(fmt).unwrap_err();
            assert!(
                matches!(err, ReleasaurusError::InvalidConfig(_)),
                "{fmt:?}"
            );
        }
    }

    #[test]
    fn version_type_from_str_rejects_irreversible_strftime_formats() {
        for fmt in ["%Q", "%Y.%A", "%m.%d", "%H.%M", "%Y.%I", "%Y.%W", "%s"] {
            let err = VersionType::from_str(fmt).unwrap_err();
            assert!(
                matches!(err, ReleasaurusError::InvalidConfig(_)),
                "{fmt:?}"
            );
        }
    }

    #[test]
    fn version_type_from_str_rejects_formats_that_are_not_git_tag_names() {
        for fmt in [
            "%Y %m",
            "%T",
            "%c",
            "%Y:%m",
            "%Y..%m",
            "%Y[%m]",
            "%Y.%m.",
            "%Y.%m.lock",
            "%Y/%m.lock/%d",
            "%Y@{%m",
            "%Y*",
            "%Y?",
            "%Y~%m",
            "%Y^%m",
            "%Y\\%m",
            "%Y//%m",
            "%Y/",
            "%Y\t%m",
            // space-padded: reject formats that render with spaces
            "%Y.%m.%d.%k",
            "%Y.%m.%d.%_H",
            "%Y.%_m.%d",
            "%Y.%_j",
        ] {
            let err = VersionType::from_str(fmt).unwrap_err();
            assert!(
                matches!(err, ReleasaurusError::InvalidConfig(_)),
                "{fmt:?}"
            );
        }
    }

    #[test]
    fn version_type_serde_round_trips_and_rejects_typos() {
        #[derive(Serialize, Deserialize)]
        struct Wrapper {
            version_type: VersionType,
        }

        let w: Wrapper =
            toml::from_str(r#"version_type = "%Y.%m.%d""#).unwrap();
        assert_eq!(w.version_type, VersionType::Strftime("%Y.%m.%d".into()));
        assert_eq!(
            toml::to_string(&w).unwrap().trim(),
            r#"version_type = "%Y.%m.%d""#
        );

        let w: Wrapper =
            toml::from_str(r#"version_type = "year.month.day""#).unwrap();
        assert_eq!(w.version_type, VersionType::SemanticDate);

        assert!(
            toml::from_str::<Wrapper>(r#"version_type = "year.month.dya""#)
                .is_err()
        );
    }

    #[test]
    fn parse_strftime_defaults_omitted_fields_to_start_of_period() {
        let cases = [
            ("2026.10.05", "%Y.%m.%d", (2026, 10, 5, 0, 0, 0)),
            ("2026.10", "%Y.%m", (2026, 10, 1, 0, 0, 0)),
            ("2026", "%Y", (2026, 1, 1, 0, 0, 0)),
            ("26.10.5.1307", "%y.%-m.%-d.%H%M", (2026, 10, 5, 13, 7, 0)),
            ("2026.278", "%Y.%j", (2026, 10, 5, 0, 0, 0)),
        ];
        for (raw, fmt, (y, mo, d, h, mi, s)) in cases {
            assert_eq!(
                parse_strftime(raw, fmt).unwrap(),
                (Utc.with_ymd_and_hms(y, mo, d, h, mi, s).unwrap(), None),
                "{raw:?} / {fmt:?}"
            );
        }
    }

    /// Tags made under `year.month.day` are unpadded.
    #[test]
    fn parse_strftime_accepts_unpadded_fields() {
        assert_eq!(
            parse_strftime("2026.10.5", "%Y.%m.%d").unwrap(),
            (Utc.with_ymd_and_hms(2026, 10, 5, 0, 0, 0).unwrap(), None)
        );
    }

    #[test]
    fn parse_strftime_rejects_names_not_matching_format() {
        for raw in ["2026-10-05", "2026.10", "2026.10.05-rc.1", "abc"] {
            assert!(parse_strftime(raw, "%Y.%m.%d").is_err(), "{raw:?}");
        }
    }

    #[test]
    fn parse_strftime_returns_the_release_counter() {
        let date = Utc.with_ymd_and_hms(2026, 10, 5, 0, 0, 0).unwrap();
        for (raw, fmt, count) in [
            ("2026.10.05.1", "%Y.%m.%d.%i", 1),
            ("2026.10.05.12", "%Y.%m.%d.%i", 12),
            ("26.10.5.3", "%y.%-m.%-d.%i", 3),
            ("20261005.2", "%Y%m%d.%i", 2),
        ] {
            assert_eq!(
                parse_strftime(raw, fmt).unwrap(),
                (date, Some(count)),
                "{raw:?} / {fmt:?}"
            );
        }
    }

    /// Tags made before `.%i` was added to the format.
    #[test]
    fn parse_strftime_accepts_names_without_the_release_counter() {
        for (raw, fmt) in [
            ("2026.10.05", "%Y.%m.%d.%i"),
            ("26.10.5", "%y.%-m.%-d.%i"),
            ("20261005", "%Y%m%d.%i"),
        ] {
            assert_eq!(
                parse_strftime(raw, fmt).unwrap(),
                (Utc.with_ymd_and_hms(2026, 10, 5, 0, 0, 0).unwrap(), None),
                "{raw:?} / {fmt:?}"
            );
        }
    }

    #[test]
    fn parse_strftime_rejects_malformed_release_counters() {
        for raw in ["2026.10.05.", "2026.10.05.+1", "2026.10.05.x", "2026.10"] {
            assert!(parse_strftime(raw, "%Y.%m.%d.%i").is_err(), "{raw:?}");
        }
    }
}
