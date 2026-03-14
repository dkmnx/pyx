//! Parse model metadata from pi-mono's models.generated.ts

use crate::error::{PyxError, Result};
use regex::Regex;
use std::collections::HashMap;

static PROVIDER_FIELD_PATTERN: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r#"provider:\s*\"([^\"]+)\""#).expect("valid provider regex")
});
static MODEL_ID_PATTERN: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r#"id:\s*\"([^\"]+)\""#).expect("valid model id regex")
});
static PROVIDER_SECTION_PATTERN: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r#"^\"([a-z][a-z0-9-]*)\":\s*\{\s*$"#).expect("valid provider section regex")
});

/// Parse models from pi-mono models.generated.ts content.
pub fn parse_models(content: &str) -> Result<HashMap<String, Vec<String>>> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_provider = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("//")
            || trimmed.starts_with("import")
            || trimmed.starts_with("export")
        {
            continue;
        }

        if let Some(section_match) = PROVIDER_SECTION_PATTERN.captures(trimmed) {
            if let Some(provider_match) = section_match.get(1) {
                current_provider = provider_match.as_str().to_string();
                result.entry(current_provider.clone()).or_default();
            }
            continue;
        }

        if let Some(provider_match) = PROVIDER_FIELD_PATTERN.captures(trimmed) {
            if let Some(provider) = provider_match.get(1) {
                current_provider = provider.as_str().to_string();
                result.entry(current_provider.clone()).or_default();
            }
            continue;
        }

        if current_provider.is_empty() {
            continue;
        }

        if let Some(model_match) = MODEL_ID_PATTERN.captures(trimmed) {
            if let Some(model_id_match) = model_match.get(1) {
                let model_id = model_id_match.as_str().to_string();
                let models = result.entry(current_provider.clone()).or_default();
                if !models.iter().any(|existing| existing == &model_id) {
                    models.push(model_id);
                }
            }
        }
    }

    result.retain(|_, models| !models.is_empty());

    if result.is_empty() {
        return Err(PyxError::Validation(
            "No models found in content".to_string(),
        ));
    }

    Ok(result)
}

/// Legacy parser helper that flattens all parsed model IDs.
pub fn parse_model_data(data: &str) -> Result<Vec<String>> {
    let parsed = parse_models(data)?;
    let mut flattened = Vec::new();
    for models in parsed.values() {
        flattened.extend(models.iter().cloned());
    }
    Ok(flattened)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_provider_field_blocks() {
        let content = r#"
model "foo" {
  provider: "anthropic"
  id: "claude-3-5-sonnet-20241022"
}
"#;

        let parsed = parse_models(content).expect("should parse models");
        assert_eq!(
            parsed.get("anthropic"),
            Some(&vec!["claude-3-5-sonnet-20241022".to_string()])
        );
    }

    #[test]
    fn parses_provider_sections() {
        let content = r#"
"openai": {
  chat: {
    id: "openai/gpt-4.1"
  }
}
"#;

        let parsed = parse_models(content).expect("should parse provider sections");
        assert_eq!(
            parsed.get("openai"),
            Some(&vec!["openai/gpt-4.1".to_string()])
        );
    }

    #[test]
    fn deduplicates_models_per_provider() {
        let content = r#"
model "a" {
  provider: "openai"
  id: "openai/gpt-4"
}
model "b" {
  provider: "openai"
  id: "openai/gpt-4"
}
"#;

        let parsed = parse_models(content).expect("should parse with dedupe");
        assert_eq!(parsed.get("openai").map(std::vec::Vec::len), Some(1));
    }

    #[test]
    fn ignores_comments_and_import_export_lines() {
        let content = r#"
// generated file
import { something } from "x";
export const models = {
"anthropic": {
  id: "anthropic/claude-3"
}
}
"#;

        let parsed = parse_models(content).expect("should parse while ignoring noise");
        assert_eq!(
            parsed.get("anthropic"),
            Some(&vec!["anthropic/claude-3".to_string()])
        );
    }

    #[test]
    fn returns_error_for_empty_or_unparseable_content() {
        let err = parse_models("\n\n").expect_err("should fail when nothing parseable is found");
        assert!(matches!(err, PyxError::Validation(_)));
    }
}
