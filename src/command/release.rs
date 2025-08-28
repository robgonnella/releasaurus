use color_eyre::eyre::Result;

use crate::cli;

pub fn execute(_args: &cli::Args) -> Result<()> {
    // - Look for closed PRs with the pending label. If found block this release
    // - Should we make sure this is a release commit by parsing the message?
    // - Tag commit for each release and push tags`
    // - Create Release with changelog as notes
    // - Comment on merged release pr and replace labels with tagged labels
    Ok(())
}
