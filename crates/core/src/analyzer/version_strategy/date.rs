use chrono::{Datelike, Timelike, Utc};
use semver::Version;

use crate::{
    analyzer::version_strategy::{context::Context, traits::VersionStrategy},
    result::Result,
};

/// Current UTC date and time, split into the fields the date-based
/// strategies build versions out of.
pub(crate) struct DateParts {
    pub year: u64,
    pub month: u64,
    pub day: u64,
    pub hour: u64,
    pub minute: u64,
    pub second: u64,
    pub micro: u64,
}

impl DateParts {
    pub fn now() -> Self {
        let date = Utc::now();

        Self {
            // Every date `Utc::now()` can return is well past year 0, so
            // dropping the sign here is lossless.
            year: u64::from(date.year().unsigned_abs()),
            month: date.month().into(),
            day: date.day().into(),
            hour: date.hour().into(),
            minute: date.minute().into(),
            second: date.second().into(),
            micro: date.timestamp_subsec_micros().into(),
        }
    }

    /// `hour.minute.second`, zero-padded so a rendered tag sorts the same
    /// lexicographically as it does numerically — `git tag --list` and forge
    /// tag listings sort as text, unlike semver's own numeric comparison of
    /// build metadata. Leading zeros are legal in build metadata; only
    /// prerelease identifiers forbid them.
    pub fn time_build_metadata(&self) -> String {
        format!("{:02}.{:02}.{:02}", self.hour, self.minute, self.second)
    }
}

#[derive(Default)]
pub struct DateVersionStrategy;

impl VersionStrategy for DateVersionStrategy {
    fn calculate_next_version(&self, _ctx: &Context) -> Result<Version> {
        let parts = DateParts::now();
        Ok(Version::new(parts.year, parts.month, parts.day))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_parts_now() {
        let parts = DateParts::now();

        assert_eq!(parts.year, Utc::now().year() as u64);
        assert!((1..=12).contains(&parts.month));
        assert!((1..=31).contains(&parts.day));
        assert!(parts.hour <= 23);
        assert!(parts.minute <= 59);
        assert!(parts.second <= 59);
    }

    /// Segments are padded to two digits so tag names sort as text in the
    /// same order they sort numerically.
    #[test]
    fn test_time_build_metadata_is_zero_padded() {
        let parts = DateParts {
            year: 2026,
            month: 7,
            day: 26,
            hour: 9,
            minute: 5,
            second: 3,
            micro: 42,
        };

        assert_eq!(parts.time_build_metadata(), "09.05.03");
    }
}
