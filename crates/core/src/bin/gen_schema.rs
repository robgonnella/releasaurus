use releasaurus_core::{config::Config, result::Result};
use schemars::schema_for;
use std::fs;

fn main() -> Result<()> {
    let schema = schema_for!(Config);
    let schema_string = serde_json::to_string_pretty(&schema)?;
    fs::write("schema/schema.json", schema_string.as_bytes())?;
    Ok(())
}
