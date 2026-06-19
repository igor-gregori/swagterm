use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct SwaggerSpec {
    pub info: Info,
    #[serde(default)]
    pub host: String,
    #[serde(rename = "basePath", default)]
    pub base_path: String,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub paths: IndexMap<String, IndexMap<String, Operation>>,
    #[serde(default)]
    pub definitions: HashMap<String, Schema>,
}

#[derive(Debug, Deserialize)]
pub struct Info {
    pub title: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Tag {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Operation {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "operationId", default)]
    pub operation_id: Option<String>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    #[serde(default)]
    pub responses: IndexMap<String, Response>,
    #[serde(default)]
    pub consumes: Vec<String>,
    #[serde(default)]
    pub produces: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(rename = "type", default)]
    pub param_type: Option<String>,
    #[serde(default)]
    pub schema: Option<Schema>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Response {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub schema: Option<Schema>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Schema {
    #[serde(rename = "type", default)]
    pub schema_type: Option<String>,
    #[serde(default)]
    pub properties: IndexMap<String, Schema>,
    #[serde(rename = "$ref", default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub items: Option<Box<Schema>>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "enum", default)]
    pub enum_values: Vec<serde_json::Value>,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub format: Option<String>,
}

pub fn parse_file(path: &Path) -> Result<SwaggerSpec, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "json" => serde_json::from_str(&content).map_err(|e| format!("JSON parse error: {e}")),
        "yaml" | "yml" => {
            serde_yaml::from_str(&content).map_err(|e| format!("YAML parse error: {e}"))
        }
        _ => Err(format!("Unsupported file extension: {ext}")),
    }
}
