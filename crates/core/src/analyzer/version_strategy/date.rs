use chrono::{Datelike, Timelike, Utc};
use color_eyre::eyre::eyre;
use semver::Version;

use crate::{
    analyzer::version_strategy::{context::Context, traits::VersionStrategy},
    result::{ReleasaurusError, Result},
};

pub(crate) fn get_date_parts() -> Result<[u64; 7]> {
    let date = Utc::now();
    let year = u64::try_from(date.year()).map_err(|e| {
        ReleasaurusError::Other(eyre!(
            "failed to parse year from current date: {}",
            e
        ))
    })?;

    let month: u64 = date.month().into();
    let day: u64 = date.day().into();
    let hr: u64 = date.hour().into();
    let min: u64 = date.minute().into();
    let sec: u64 = date.second().into();
    let micro: u64 = date.timestamp_subsec_micros().into();

    Ok([year, month, day, hr, min, sec, micro])
}

#[derive(Default)]
pub struct DateVersionStrategy;

impl VersionStrategy for DateVersionStrategy {
    fn calculate_next_version(&self, _ctx: &Context) -> Result<Version> {
        let [year, month, day, ..] = get_date_parts()?;
        Ok(Version::new(year, month, day))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_date_parts() {
        let [year, month, day, hr, min, sec, _micro] =
            get_date_parts().unwrap();

        assert_eq!(year, Utc::now().year() as u64);
        assert!((1..=12).contains(&month));
        assert!((1..=31).contains(&day));
        assert!(hr <= 23);
        assert!(min <= 59);
        assert!(sec <= 60); // allow leap second
    }
}
