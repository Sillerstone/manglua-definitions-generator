use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;
use std::fs::read_to_string;
use std::path::Path;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Dump {
    types: Vec<Type>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Type {
    lua_name: String,
    clr_name: String,
    constructors: Vec<Constructor>,
    properties: Vec<Property>,
    fields: Vec<Field>,
    methods: Vec<Method>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Constructor {
    has_param_array: bool,
    parameters: Vec<Parameter>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Parameter {
    name: String,
    r#type: String,
    optional: bool,
    // Meaning: is current parameter represents parameters array for reflection methods
    is_params: bool,
    default: Option<Value>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Property {
    name: String,
    r#type: String,
    r#static: bool,
    can_read: bool,
    can_write: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Field {
    name: String,
    r#type: String,
    r#static: bool,
    read_only: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Method {
    name: String,
    r#static: bool,
    return_type: String,
    has_param_array: bool, // Should be removed
    parameters: Vec<Parameter>,
}

pub fn generate_definitions(dump: &Path, output: &Path) -> Result<()> {
    let dump: Dump = serde_json::from_str(read_to_string(dump)?.as_str())?;
    println!("{dump:?}");
    Ok(())
}
