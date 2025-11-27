use releasaurus::config;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(config::Config);
    let schema_string = serde_json::to_string_pretty(&schema).unwrap();
    println!("{}", schema_string);
}
