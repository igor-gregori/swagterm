use crate::swagger::ApiSpec;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct Warning {
    pub path: String,
    pub message: String,
}

pub fn validate(spec: &ApiSpec) -> Vec<Warning> {
    let mut warnings = Vec::new();
    let def_names: HashSet<&str> = spec.definitions.keys().map(|s| s.as_str()).collect();

    let mut operation_ids: Vec<(&str, &str)> = Vec::new(); // (op_id, path)

    for (path, methods) in &spec.paths {
        for (method, op) in methods {
            let loc = format!("{} {}", method.to_uppercase(), path);

            // Missing responses
            if op.responses.is_empty() {
                warnings.push(Warning { path: loc.clone(), message: "No responses defined".into() });
            }

            // Duplicate operation IDs
            if let Some(op_id) = &op.operation_id {
                if let Some((_, prev_path)) = operation_ids.iter().find(|(id, _)| id == &op_id.as_str()) {
                    warnings.push(Warning {
                        path: loc.clone(),
                        message: format!("Duplicate operationId '{op_id}' (also in {prev_path})"),
                    });
                }
                operation_ids.push((op_id.as_str(), Box::leak(loc.clone().into_boxed_str())));
            }

            // Missing path params
            let path_params: Vec<&str> = path
                .split('/')
                .filter(|s| s.starts_with('{') && s.ends_with('}'))
                .map(|s| &s[1..s.len() - 1])
                .collect();
            for pp in &path_params {
                let defined = op.parameters.iter().any(|p| p.location == "path" && p.name == *pp);
                if !defined {
                    warnings.push(Warning {
                        path: loc.clone(),
                        message: format!("Path parameter '{pp}' not defined in parameters"),
                    });
                }
            }

            // Parameters without type
            for p in &op.parameters {
                if p.location != "body" && p.param_type.is_none() && p.schema.is_none() {
                    warnings.push(Warning {
                        path: loc.clone(),
                        message: format!("Parameter '{}' has no type", p.name),
                    });
                }
            }

            // Unresolved $ref in parameters
            for p in &op.parameters {
                if let Some(schema) = &p.schema {
                    check_refs(schema, &def_names, &loc, &mut warnings);
                }
            }

            // Unresolved $ref in responses
            for (_, resp) in &op.responses {
                if let Some(schema) = &resp.schema {
                    check_refs(schema, &def_names, &loc, &mut warnings);
                }
            }
        }
    }

    warnings
}

fn check_refs(schema: &crate::swagger::Schema, defs: &HashSet<&str>, loc: &str, warnings: &mut Vec<Warning>) {
    if let Some(ref_path) = &schema.reference {
        let ref_name = ref_path
            .strip_prefix("#/definitions/")
            .or_else(|| ref_path.strip_prefix("#/components/schemas/"))
            .unwrap_or(ref_path);
        if !defs.contains(ref_name) {
            warnings.push(Warning {
                path: loc.to_string(),
                message: format!("Unresolved $ref: {ref_path}"),
            });
        }
    }
    if let Some(items) = &schema.items {
        check_refs(items, defs, loc, warnings);
    }
    for (_, prop) in &schema.properties {
        check_refs(prop, defs, loc, warnings);
    }
}
