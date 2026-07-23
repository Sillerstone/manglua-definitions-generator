use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{File, read_to_string};
use std::io::Write;
use std::ops::Add;
use std::path::Path;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Dump {
    type_count: u8,
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
    parameters: Vec<Parameter>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Parameter {
    name: String,
    r#type: String,
    optional: bool,
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
    return_type: String,
    parameters: Vec<Parameter>,
}

pub fn generate_definitions(dump: &Path, output: &Path) -> Result<()> {
    let dump: Dump = serde_json::from_str(read_to_string(dump)?.as_str())?;
    let mut types_mapping: HashMap<&String, &String> =
        HashMap::with_capacity(dump.type_count as usize);
    let native_types = HashMap::from([
        ("System.Byte".to_string(), "integer".to_string()),
        ("System.Int32".to_string(), "integer".to_string()),
        ("System.Int64".to_string(), "integer".to_string()),
        ("System.Single".to_string(), "number".to_string()),
        ("System.Boolean".to_string(), "boolean".to_string()),
        ("System.String".to_string(), "string".to_string()),
        ("System.Object".to_string(), "any".to_string()),
        ("System.Action".to_string(), "fun()".to_string()),
        ("MangLua.Proxies.LuaFunctionRef".to_string(), "fun()".to_string()),
    ]);
    for (key, value) in &native_types {
        types_mapping.insert(key, value);
    }
    for t in &dump.types {
        types_mapping.insert(&t.clr_name, &t.lua_name);
    }
    for t in &dump.types {
        let mut definition = "---@meta\n\n\n---@class ".to_string();
        definition += format!("{}\n", t.lua_name).as_str();
        for field in &t.fields {
            definition += format!(
                "---@field {} {} {}\n",
                field.name,
                types_mapping.get_or_fallback(&field.r#type),
                get_field_description(field)
            )
            .as_str();
        }
        for property in &t.properties {
            definition += format!(
                "---@field {} {} {}\n",
                property.name,
                types_mapping.get_or_fallback(&property.r#type),
                get_property_description(property)
            )
            .as_str();
        }
        definition += (format!("{} = ", t.lua_name) + "{}\n\n").as_str();
        for constructor in &t.constructors {
            for param in &constructor.parameters {
                definition += handle_param(param, &types_mapping).as_str();
            }
            definition += format!("---@return {}\n", t.lua_name).as_str();
            definition += format!(
                "function {}.new({}) end\n\n",
                t.lua_name,
                get_method_params(&constructor.parameters)
            )
            .as_str();
        }
        for method in &t.methods {
            for param in &method.parameters {
                definition += handle_param(param, &types_mapping).as_str();
            }
            if method.return_type != "void" {
                definition += format!(
                    "---@return {}\n",
                    types_mapping.get_or_fallback(&method.return_type)
                )
                .as_str();
            }
            definition += format!(
                "function {}.{}({}) end\n\n",
                t.lua_name,
                method.name,
                get_method_params(&method.parameters)
            )
            .as_str();
        }
        File::create(output.join(format!("{}.lua", t.lua_name)))?
            .write_all(definition.trim().as_bytes())?;
    }
    Ok(())
}

trait F {
    fn get_or_fallback<'a>(&'a self, key: &'a String) -> String;
}

impl F for HashMap<&String, &String> {
    fn get_or_fallback<'a>(&'a self, key: &'a String) -> String {
        if key.ends_with("[]") {
            let mut new_key = key.clone();
            new_key.pop();
            new_key.pop();
            let t = self.get(&new_key).map(|s| { (*s).clone().add("[]") });
            return t.unwrap_or(key.clone());
        } else if key.starts_with("System.Collections.Generic.List<") {
            let parts: Vec<&str> = key.split("<").collect();
            let mut inner_type = parts[1].to_string();
            inner_type.pop();
            return (*self.get(&inner_type).unwrap_or(&key)).clone() + "[]"
        } else if key.starts_with("System.Collections.Generic.Dictionary<") {
            let parts: Vec<&str> = key.split("<").collect();
            let parts: Vec<&str> = parts[1].split(", ").collect();
            let entry_key = *self.get(&parts[0].to_string()).unwrap_or(&key);
            let mut value_type = parts[1].to_string();
            value_type.pop();
            let entry_value = *self.get(&value_type).unwrap_or(&key);
            let mut t = "{".to_string();
            t += format!(" [{}]: {} ", entry_key, entry_value).as_str();
            t += "}";
            return t;
        } else if key.starts_with("System.Action<") {
            let parts: Vec<&str> = key.split("<").collect();
            let mut inner_type = parts[1].to_string();
            inner_type.pop();
            return format!("fun({})", *self.get(&inner_type).unwrap_or(&key));
        }
        (*self.get(key).unwrap_or(&key)).clone()
    }
}

fn handle_param(param: &Parameter, types_mapping: &HashMap<&String, &String>) -> String {
    let param_type = types_mapping.get_or_fallback(&param.r#type);
    let param_type = if param.optional {
        param_type.clone() + "?"
    } else {
        param_type.clone()
    };
    format!(
        "---@param {} {} {}\n",
        param.name,
        param_type,
        get_param_description(param)
    )
}

const DESCRIPTION_DELIMITER: &str = " | ";

fn get_field_description(field: &Field) -> String {
    let is_static = if field.r#static {
        "Static field"
    } else {
        "Non-static field"
    };
    let is_readonly = if field.read_only {
        "Read"
    } else {
        "Read/Write"
    };
    format!("{is_static}{DESCRIPTION_DELIMITER}{is_readonly}")
}

fn get_property_description(property: &Property) -> String {
    let is_static = if property.r#static {
        "Static property"
    } else {
        "Non-static property"
    };
    let get_set = if property.can_read && property.can_write {
        "Read/Write"
    } else if property.can_read {
        "Read"
    } else {
        "Write"
    };
    format!("{is_static}{DESCRIPTION_DELIMITER}{get_set}")
}

fn get_param_description(param: &Parameter) -> String {
    if let Some(default) = &param.default {
        format!("Default value: {}", default.to_string())
    } else {
        "".to_string()
    }
}

fn get_method_params(params: &Vec<Parameter>) -> String {
    let mut result = String::new();
    for param in params {
        result += format!("{}, ", param.name).as_str();
    }
    if !params.is_empty() {
        result.pop();
        result.pop();
    }
    result
}
