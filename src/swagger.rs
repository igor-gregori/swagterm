use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

// ─── Unified model (what the app uses) ───────────────────────────────────────

#[derive(Debug)]
pub struct ApiSpec {
    pub info: Info,
    pub paths: IndexMap<String, IndexMap<String, Operation>>,
    pub definitions: HashMap<String, Schema>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Info {
    pub title: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub tags: Vec<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub operation_id: Option<String>,
    pub parameters: Vec<Parameter>,
    pub responses: IndexMap<String, Response>,
    pub consumes: Vec<String>,
    pub produces: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub location: String,
    pub description: Option<String>,
    pub required: bool,
    pub param_type: Option<String>,
    pub schema: Option<Schema>,
}

#[derive(Debug, Clone)]
pub struct Response {
    pub description: Option<String>,
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

// ─── Swagger 2.0 raw model ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct Swagger2 {
    info: Info,
    #[serde(default)]
    paths: IndexMap<String, IndexMap<String, Swagger2Operation>>,
    #[serde(default)]
    definitions: HashMap<String, Schema>,
}

#[derive(Deserialize)]
struct Swagger2Operation {
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "operationId", default)]
    operation_id: Option<String>,
    #[serde(default)]
    parameters: Vec<Swagger2Parameter>,
    #[serde(default)]
    responses: IndexMap<String, Swagger2Response>,
    #[serde(default)]
    consumes: Vec<String>,
    #[serde(default)]
    produces: Vec<String>,
}

#[derive(Deserialize)]
struct Swagger2Parameter {
    name: String,
    #[serde(rename = "in")]
    location: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    required: bool,
    #[serde(rename = "type", default)]
    param_type: Option<String>,
    #[serde(default)]
    schema: Option<Schema>,
}

#[derive(Deserialize)]
struct Swagger2Response {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    schema: Option<Schema>,
}

// ─── OpenAPI 3.x raw model ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct OpenApi3 {
    info: Info,
    #[serde(default)]
    paths: IndexMap<String, IndexMap<String, OpenApi3Operation>>,
    #[serde(default)]
    components: Option<OpenApi3Components>,
}

#[derive(Deserialize)]
struct OpenApi3Components {
    #[serde(default)]
    schemas: HashMap<String, Schema>,
}

#[derive(Deserialize)]
struct OpenApi3Operation {
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "operationId", default)]
    operation_id: Option<String>,
    #[serde(default)]
    parameters: Vec<OpenApi3Parameter>,
    #[serde(rename = "requestBody", default)]
    request_body: Option<OpenApi3RequestBody>,
    #[serde(default)]
    responses: IndexMap<String, OpenApi3Response>,
}

#[derive(Deserialize)]
struct OpenApi3Parameter {
    name: String,
    #[serde(rename = "in")]
    location: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    schema: Option<Schema>,
}

#[derive(Deserialize)]
struct OpenApi3RequestBody {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    content: IndexMap<String, OpenApi3MediaType>,
}

#[derive(Deserialize)]
struct OpenApi3MediaType {
    #[serde(default)]
    schema: Option<Schema>,
}

#[derive(Deserialize)]
struct OpenApi3Response {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    content: Option<IndexMap<String, OpenApi3MediaType>>,
}

// ─── Conversion ─────────────────────────────────────────────────────────────

impl From<Swagger2> for ApiSpec {
    fn from(s: Swagger2) -> Self {
        let paths = s
            .paths
            .into_iter()
            .map(|(path, methods)| {
                let ops = methods
                    .into_iter()
                    .map(|(method, op)| {
                        (
                            method,
                            Operation {
                                tags: op.tags,
                                summary: op.summary,
                                description: op.description,
                                operation_id: op.operation_id,
                                parameters: op
                                    .parameters
                                    .into_iter()
                                    .map(|p| Parameter {
                                        name: p.name,
                                        location: p.location,
                                        description: p.description,
                                        required: p.required,
                                        param_type: p.param_type,
                                        schema: p.schema,
                                    })
                                    .collect(),
                                responses: op
                                    .responses
                                    .into_iter()
                                    .map(|(code, r)| {
                                        (code, Response { description: r.description, schema: r.schema })
                                    })
                                    .collect(),
                                consumes: op.consumes,
                                produces: op.produces,
                            },
                        )
                    })
                    .collect();
                (path, ops)
            })
            .collect();
        ApiSpec { info: s.info, paths, definitions: s.definitions }
    }
}

impl From<OpenApi3> for ApiSpec {
    fn from(s: OpenApi3) -> Self {
        let definitions = s
            .components
            .map(|c| c.schemas)
            .unwrap_or_default();

        let paths = s
            .paths
            .into_iter()
            .map(|(path, methods)| {
                let ops = methods
                    .into_iter()
                    .map(|(method, op)| {
                        let mut parameters: Vec<Parameter> = op
                            .parameters
                            .into_iter()
                            .map(|p| Parameter {
                                name: p.name,
                                location: p.location,
                                description: p.description,
                                required: p.required,
                                param_type: p.schema.as_ref().and_then(|s| s.schema_type.clone()),
                                schema: p.schema,
                            })
                            .collect();

                        let mut consumes = Vec::new();
                        // Convert requestBody to a body parameter
                        if let Some(rb) = op.request_body {
                            for (media_type, _) in &rb.content {
                                consumes.push(media_type.clone());
                            }
                            let schema = rb
                                .content
                                .into_values()
                                .next()
                                .and_then(|mt| mt.schema);
                            parameters.push(Parameter {
                                name: "body".into(),
                                location: "body".into(),
                                description: rb.description,
                                required: rb.required,
                                param_type: None,
                                schema,
                            });
                        }

                        let mut produces = Vec::new();
                        let responses = op
                            .responses
                            .into_iter()
                            .map(|(code, r)| {
                                let schema = r.content.and_then(|c| {
                                    for (media_type, _) in &c {
                                        if !produces.contains(media_type) {
                                            produces.push(media_type.clone());
                                        }
                                    }
                                    c.into_values().next().and_then(|mt| mt.schema)
                                });
                                (code, Response { description: r.description, schema })
                            })
                            .collect();

                        (
                            method,
                            Operation {
                                tags: op.tags,
                                summary: op.summary,
                                description: op.description,
                                operation_id: op.operation_id,
                                parameters,
                                responses,
                                consumes,
                                produces,
                            },
                        )
                    })
                    .collect();
                (path, ops)
            })
            .collect();
        ApiSpec { info: s.info, paths, definitions }
    }
}

// ─── Parser ─────────────────────────────────────────────────────────────────

pub fn parse_file(path: &Path) -> Result<ApiSpec, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    parse_content(&content, ext)
}

fn parse_content(content: &str, ext: &str) -> Result<ApiSpec, String> {
    // Detect version by peeking at the JSON/YAML
    let value: serde_json::Value = match ext {
        "json" => serde_json::from_str(content).map_err(|e| format!("JSON parse error: {e}"))?,
        "yaml" | "yml" => serde_yaml::from_str(content).map_err(|e| format!("YAML parse error: {e}"))?,
        _ => return Err(format!("Unsupported file extension: {ext}")),
    };

    if value.get("swagger").and_then(|v| v.as_str()).is_some() {
        // Swagger 2.0
        let spec: Swagger2 = serde_json::from_value(value).map_err(|e| format!("Swagger 2.0 parse error: {e}"))?;
        Ok(spec.into())
    } else if value.get("openapi").and_then(|v| v.as_str()).is_some() {
        // OpenAPI 3.x
        let spec: OpenApi3 = serde_json::from_value(value).map_err(|e| format!("OpenAPI 3.x parse error: {e}"))?;
        Ok(spec.into())
    } else {
        Err("Could not detect spec version. Expected 'swagger' or 'openapi' field.".into())
    }
}
