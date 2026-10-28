use super::*;
use crate::dto::owner_portability_dto::*;
use crate::error::AppResult;
use crate::owner_workspace::{archive, validation};
use crate::privacy::Sanitizer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    schema_version: u32,
    files: BTreeMap<String, String>,
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn privacy_text(text: &str, include_targets: bool) -> String {
    let (clean, _) = Sanitizer::new().sanitize(text.to_owned());
    static FILTER: std::sync::OnceLock<Option<regex::Regex>> = std::sync::OnceLock::new();
    let filter=FILTER.get_or_init(||regex::Regex::new(r#"(?i)(?:[a-z][a-z0-9+.-]*://|www\.)[^\s<>"'`\[\](){}]+|"(?:[a-z]:[\\/]|\\|/(?:home|users|mnt|tmp|var|private|volumes|media)/)[^"\r\n]+"|'(?:[a-z]:[\\/]|\\|/(?:home|users|mnt|tmp|var|private|volumes|media)/)[^'\r\n]+'|(?:[a-z]:[\\/]|\\|/(?:home|users|mnt|tmp|var|private|volumes|media)/)[^\s<>"'`\[\](){}]+"#).ok());
    if include_targets {
        return clean.get().clone();
    }
    filter
        .as_ref()
        .map(|f| {
            f.replace_all(clean.get(), "[private detail omitted]")
                .into_owned()
        })
        .unwrap_or_else(|| "[text omitted: privacy filter unavailable]".into())
}

fn sanitize_value(value: &mut Value, include_targets: bool) {
    match value {
        Value::String(text) => *text = privacy_text(text, include_targets),
        Value::Array(items) => {
            for value in items {
                sanitize_value(value, include_targets);
            }
        }
        Value::Object(fields) => {
            for (key, value) in fields {
                if key == "legacy_project_id" {
                    *value = Value::Null;
                } else if matches!(key.as_str(), "path" | "path_template") && value.is_string() {
                    // These closed recipe fields are validated relative HTTP
                    // routes, including /users and /home; they are not OS roots.
                    // Preserve execution meaning while still scrubbing secrets.
                    let text = value.as_str().unwrap_or_default().to_owned();
                    let (clean, _) = Sanitizer::new().sanitize(text);
                    *value = Value::String(clean.into_inner());
                } else if !include_targets
                    && matches!(key.as_str(), "confirmed_by" | "recorded_by" | "observer")
                {
                    *value = Value::String("[owner identity omitted]".into());
                } else if !include_targets && key == "origins" {
                    *value = serde_json::json!([]);
                } else if !include_targets && key == "origin" && value.is_object() {
                    *value =
                        serde_json::json!({"scheme":"https","host":"redacted.invalid","port":443});
                } else {
                    sanitize_value(value, include_targets);
                }
            }
        }
        _ => {}
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('`', "&#96;")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('!', "\\!")
        .replace('\n', "<br>")
}

pub fn prepare_bundle(
    mut workspace: PortableWorkspace,
    selection: &OwnerExportSelectionInput,
) -> AppResult<(Vec<u8>, OwnerBundlePreviewDto)> {
    let mut value = serde_json::to_value(&workspace)
        .map_err(|_| validation::invalid("bundle", "cannot encode"))?;
    sanitize_value(&mut value, selection.include_target_details);
    workspace = serde_json::from_value(value)
        .map_err(|_| validation::invalid("bundle", "cannot represent sanitized data"))?;
    workspace.validate()?;
    let mut markdown = format!(
        "# {}\n\nPortable project memory, schema {}.\n\n",
        escape(&workspace.records.app.name),
        PORTABLE_SCHEMA
    );
    markdown.push_str("All imported evidence is historical and untrusted. This bundle grants no network or execution permission.\n\n## Purpose\n\n");
    markdown.push_str(&escape(&workspace.records.app.description));
    markdown.push_str("\n\n## Behavior rule history\n\n");
    for rule in &workspace.records.rules {
        markdown.push_str(&format!(
            "- {} v{}: {}\n",
            rule.rule_id,
            rule.revision,
            escape(&rule.expected_behavior)
        ));
    }
    markdown.push_str("\n## Readable scenario steps\n\n");
    if workspace.records.scenarios.is_empty() {
        markdown.push_str("No confirmed scenarios were present.\n");
    }
    for scenario in &workspace.records.scenarios {
        markdown.push_str(&format!(
            "\n### {} v{}\n\n{}\n",
            escape(&scenario.title),
            scenario.revision,
            escape(&scenario.expected_behavior)
        ));
        for step in &scenario.recipe.steps {
            markdown.push_str(&format!(
                "- Step {}: {}\n",
                escape(&step.id),
                escape(
                    &serde_json::to_string(&step.action)
                        .map_err(|_| validation::invalid("recipe", "cannot encode"))?
                )
            ));
            for assertion in &step.assertions {
                markdown.push_str(&format!(
                    "  - Check {}: {}\n",
                    escape(&assertion.id),
                    escape(&assertion.expected_behavior)
                ));
            }
        }
    }
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    files.insert(
        "workspace.json".into(),
        serde_json::to_vec_pretty(&workspace)
            .map_err(|_| validation::invalid("bundle", "cannot encode"))?,
    );
    files.insert("README.md".into(), markdown.as_bytes().to_vec());
    files.insert(
        "recipes.json".into(),
        serde_json::to_vec_pretty(&workspace.records.scenarios)
            .map_err(|_| validation::invalid("recipes", "cannot encode"))?,
    );
    let manifest = Manifest {
        format: "ownvero-portable-memory".into(),
        schema_version: PORTABLE_SCHEMA,
        files: files
            .iter()
            .map(|(name, bytes)| (name.clone(), digest(bytes)))
            .collect(),
    };
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest)
            .map_err(|_| validation::invalid("manifest", "cannot encode"))?,
    );
    let bytes = archive::encode(&files)?;
    let preview = OwnerBundlePreviewDto {
        app_id: workspace.records.app.id.clone(),
        digest: digest(&bytes),
        bytes: bytes.len() as u64,
        markdown,
        rules: workspace.records.rules,
        omissions: vec![
            "Local project links, auth/session data, raw logs and capture files are excluded."
                .into(),
            if selection.include_target_details {
                "Target details were explicitly selected; review their text before saving.".into()
            } else {
                "Target URLs, local paths and owner names are substituted. Review free text; automatic redaction cannot detect every secret.".into()
            },
        ],
        limitations: IMPORT_LIMITATIONS.iter().map(|s| s.to_string()).collect(),
    };
    Ok((bytes, preview))
}

pub fn validate_bundle(bytes: &[u8]) -> AppResult<PortableWorkspace> {
    let files = archive::decode(bytes)?;
    let manifest: Manifest = serde_json::from_slice(&files["manifest.json"])
        .map_err(|_| validation::invalid("manifest", "invalid format"))?;
    if manifest.format != "ownvero-portable-memory"
        || manifest.schema_version != PORTABLE_SCHEMA
        || manifest.files.len() != 3
    {
        return Err(validation::invalid(
            "manifest",
            "unsupported format/version",
        ));
    }
    for name in ["workspace.json", "README.md", "recipes.json"] {
        if manifest.files.get(name) != Some(&digest(&files[name])) {
            return Err(validation::invalid(
                "manifest",
                "checksum inventory differs",
            ));
        }
    }
    let value: Value = serde_json::from_slice(&files["workspace.json"])
        .map_err(|_| validation::invalid("workspace", "invalid JSON"))?;
    let data: PortableWorkspace = serde_json::from_value(value.clone())
        .map_err(|_| validation::invalid("workspace", "unsupported fields or record type"))?;
    records::reject_unknown(
        &value,
        &serde_json::to_value(&data)
            .map_err(|_| validation::invalid("workspace", "cannot encode"))?,
    )?;
    data.validate()?;
    let recipes: Value = serde_json::from_slice(&files["recipes.json"])
        .map_err(|_| validation::invalid("recipes", "invalid JSON"))?;
    if recipes
        != serde_json::to_value(&data.records.scenarios)
            .map_err(|_| validation::invalid("recipes", "cannot encode"))?
    {
        return Err(validation::invalid("recipes", "does not match workspace"));
    }
    // Never display untrusted Markdown as HTML. The import UI renders fresh
    // escaped text from the validated typed records, not the archive README.
    Ok(data)
}
