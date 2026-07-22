use crate::config;
use anyhow::{Error, Result};
use serde::Deserialize;
use std::fs::{File, read_to_string};
use std::io::Write;
use std::path::Path;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionOnlyDump {
    defgen_version: i32,
}

pub fn download_dump(to: &Path) -> Result<()> {
    let response = reqwest::blocking::get(&config().dump_url)?;
    let bytes = response.bytes()?;

    let mut invalid_format = false;
    if let Ok(str) = str::from_utf8(bytes.as_ref()) {
        if let Some(first_char) = str.chars().next() {
            invalid_format = first_char != '{';
        }
    } else {
        invalid_format = true;
    }
    if invalid_format {
        return Err(Error::msg("Invalid dump format"));
    }

    let mut file = File::create(to)?;
    file.write_all(bytes.as_ref())?;
    Ok(())
}

pub fn get_actual_dump_version(dump: &Path) -> Result<i32> {
    let dump = read_to_string(dump)?;
    Ok(serde_json::from_str::<VersionOnlyDump>(dump.as_str())?.defgen_version)
}

pub fn get_expected_dump_version() -> Result<i32> {
    let response = reqwest::blocking::get(&config().dump_url)?;
    let bytes = response.bytes()?;

    Ok(serde_json::from_str::<VersionOnlyDump>(str::from_utf8(bytes.as_ref())?)?.defgen_version)
}
