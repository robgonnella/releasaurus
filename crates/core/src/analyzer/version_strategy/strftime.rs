use crate::{
    analyzer::version_strategy::{context::Context, traits::VersionStrategy},
    forge::request::TagVersion,
    result::Result,
};

/// Ignores commits; the version is the current UTC time. The current tag is
/// consulted only to continue a [`COUNTER_SUFFIX`] counter on the same date.
///
/// [`COUNTER_SUFFIX`]: crate::config::versioning::COUNTER_SUFFIX
pub struct StrftimeVersionStrategy {
    fmt: String,
}

impl StrftimeVersionStrategy {
    pub fn new(fmt: String) -> Self {
        Self { fmt }
    }
}

impl VersionStrategy for StrftimeVersionStrategy {
    fn calculate_next_version(&self, ctx: &Context) -> Result<TagVersion> {
        let mut next = TagVersion::strftime_now(&self.fmt)?;

        if let TagVersion::Strftime {
            date,
            count: Some(count),
            ..
        } = &mut next
            && let Some(current) = ctx.current_tag
            && let TagVersion::Strftime {
                date: current_date,
                count: Some(current_count),
                ..
            } = &current.version
            && current_date == date
        {
            *count = current_count.saturating_add(1);
        }

        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::*;
    use crate::{
        analyzer::{config::AnalyzerConfig, tests::on_stable_date},
        config::versioning::VersionType,
        forge::request::Tag,
    };

    const FMT: &str = "%Y.%m.%d.%i";

    fn next_version(fmt: &str, current_tag: Option<&Tag>) -> TagVersion {
        let config = AnalyzerConfig::default();
        let ctx = Context {
            config: &config,
            current_tag,
            commits: &[],
            short_sha: "",
            timestamp: 0,
        };
        StrftimeVersionStrategy::new(fmt.into())
            .calculate_next_version(&ctx)
            .unwrap()
    }

    fn tag(name: &str) -> Tag {
        Tag {
            name: name.into(),
            version: TagVersion::parse(
                name,
                &VersionType::Strftime(FMT.into()),
            )
            .unwrap(),
            ..Tag::default()
        }
    }

    fn count(version: &TagVersion) -> Option<u16> {
        let TagVersion::Strftime { count, .. } = version else {
            panic!("expected strftime version");
        };
        *count
    }

    #[test]
    fn counter_starts_at_one_without_a_current_tag() {
        assert_eq!(count(&next_version(FMT, None)), Some(1));
    }

    #[test]
    fn counter_continues_from_a_tag_on_the_same_date() {
        let (today, next) = on_stable_date(|today| {
            next_version(FMT, Some(&tag(&format!("{today}.3"))))
        });

        assert_eq!(next.to_string(), format!("{today}.4"));
    }

    #[test]
    fn counter_restarts_when_the_date_changes() {
        let yesterday = (Utc::now() - Duration::days(1)).format("%Y.%m.%d");
        let current = tag(&format!("{yesterday}.5"));

        let next = next_version(FMT, Some(&current));

        assert_eq!(count(&next), Some(1));
    }

    #[test]
    fn counter_restarts_after_a_tag_without_a_counter() {
        let (today, next) = on_stable_date(|today| {
            let current = tag(today);
            assert_eq!(count(&current.version), None);
            next_version(FMT, Some(&current))
        });

        assert_eq!(next.to_string(), format!("{today}.1"));
    }

    #[test]
    fn format_without_counter_has_no_count() {
        assert_eq!(count(&next_version("%Y.%m.%d", None)), None);
    }
}
