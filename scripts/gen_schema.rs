use std::fs;

use releasaurus::{config, result::Result};
use schemars::schema_for;

fn main() -> Result<()> {
    let schema = schema_for!(config::Config);
    let schema_string = serde_json::to_string_pretty(&schema).unwrap();
    fs::write("./schema/schema.json", schema_string.as_bytes())?;
    Ok(())
}
