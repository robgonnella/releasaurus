use color_eyre::eyre::{ContextCompat, Result, eyre};
use regex::Regex;

use crate::forge::types::Release;

fn parse_tag_from_notes(notes: &str) -> Result<String> {
    let tag_re = Regex::new(r"<summary>(?<tag>.+)</summary>")?;

    if let Some(captures) = tag_re.captures(notes) {
        let tag = &captures["tag"];
        return Ok(String::from(tag));
    }

    Err(eyre!("no tag found for notes"))
}

pub fn parse_pr_body(body: &str) -> Result<Vec<Release>> {
    let mut releases = vec![];

    let details_re = Regex::new(r"(?ms)<details>(?<notes>.+)")?;
    let summary_re = Regex::new(r"^<summary>.+</summary><br>")?;

    let captures = details_re.captures_iter(body);

    for cap in captures {
        let notes =
            cap.name("notes").wrap_err("failed to get release notes")?;

        let tag = parse_tag_from_notes(notes.as_str())?;

        let mut stripped =
            summary_re.replace_all(notes.as_str(), "").to_string();
        stripped = stripped.replace(r"</details>", "");

        releases.push(Release {
            tag,
            notes: stripped,
        });
    }

    Ok(releases)
}
