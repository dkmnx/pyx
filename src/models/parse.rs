//! Parse model metadata from pi-mono's models.generated.ts

use crate::error::{PyxError, Result};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static PROVIDER_FIELD_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"provider:\s*\"([^\"]+)\""#).expect("valid provider regex"));
static MODEL_ID_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"id:\s*\"([^\"]+)\""#).expect("valid model id regex"));
static PROVIDER_SECTION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\"([a-z][a-z0-9-]*)\":\s*\{\s*$"#).expect("valid provider section regex")
});

#[derive(Default)]
struct ProviderModels {
    models: Vec<String>,
    seen: HashSet<String>,
}

impl ProviderModels {
    fn push(&mut self, model_id: String) {
        if self.seen.insert(model_id.clone()) {
            self.models.push(model_id);
        }
    }
}

fn provider_name(line: &str) -> Option<&str> {
    PROVIDER_SECTION_PATTERN
        .captures(line)
        .and_then(|captures| captures.get(1))
        .map(|provider| provider.as_str())
        .or_else(|| {
            PROVIDER_FIELD_PATTERN
                .captures(line)
                .and_then(|captures| captures.get(1))
                .map(|provider| provider.as_str())
        })
}

fn model_id(line: &str) -> Option<&str> {
    MODEL_ID_PATTERN
        .captures(line)
        .and_then(|captures| captures.get(1))
        .map(|model| model.as_str())
}

/// Parse models from pi-mono models.generated.ts content.
pub fn parse_models(content: &str) -> Result<HashMap<String, Vec<String>>> {
    let mut result: HashMap<String, ProviderModels> = HashMap::new();
    let mut current_provider: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("//")
            || trimmed.starts_with("import")
            || trimmed.starts_with("export")
        {
            continue;
        }

        if let Some(provider) = provider_name(trimmed) {
            current_provider = Some(provider.to_owned());
            result.entry(provider.to_owned()).or_default();
            continue;
        }

        let Some(provider) = current_provider.as_deref() else {
            continue;
        };

        let Some(model_id) = model_id(trimmed) else {
            continue;
        };

        if let Some(models) = result.get_mut(provider) {
            models.push(model_id.to_owned());
        }
    }

    let parsed: HashMap<String, Vec<String>> = result
        .into_iter()
        .filter_map(|(provider, models)| {
            if models.models.is_empty() {
                None
            } else {
                Some((provider, models.models))
            }
        })
        .collect();

    if parsed.is_empty() {
        return Err(PyxError::Validation(
            "No models found in content".to_string(),
        ));
    }

    Ok(parsed)
}

/// Legacy parser helper that flattens all parsed model IDs.
pub fn parse_model_data(models_content: &str) -> Result<Vec<String>> {
    let parsed = parse_models(models_content)?;
    let mut providers: Vec<_> = parsed.into_iter().collect();
    providers.sort_by(|(left, _), (right, _)| left.cmp(right));

    let mut flattened = Vec::new();
    for (_, models) in providers {
        flattened.extend(models);
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
    fn parse_model_data_returns_models_in_deterministic_order() {
        let content = r#"
"openai": {
  id: "openai/gpt-4.1"
}
"anthropic": {
  id: "anthropic/claude-3-7-sonnet"
}
"#;

        for _ in 0..32 {
            let parsed = parse_model_data(content).expect("should flatten parsed models");
            assert_eq!(
                parsed,
                vec![
                    "anthropic/claude-3-7-sonnet".to_string(),
                    "openai/gpt-4.1".to_string()
                ]
            );
        }
    }

    #[test]
    fn returns_error_for_empty_or_unparseable_content() {
        let err = parse_models("\n\n").expect_err("should fail when nothing parseable is found");
        assert!(matches!(err, PyxError::Validation(_)));
    }
}
