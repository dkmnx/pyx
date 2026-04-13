//! Shared interactive prompt helpers.

use crate::error::{PyxError, Result};
use inquire::{Autocomplete, Confirm, InquireError, Password, Text};

const MAX_AUTOCOMPLETE_SUGGESTIONS: usize = 7;

pub struct SecretPromptOptions {
    pub prompt: String,
    pub helper: Option<String>,
    pub confirmation: Option<(String, String)>,
    pub empty_error: String,
}

pub fn prompt_secret(options: SecretPromptOptions) -> Result<String> {
    let SecretPromptOptions {
        prompt,
        helper,
        confirmation,
        empty_error,
    } = options;

    if let Some(helper_text) = helper {
        println!("{helper_text}");
        println!();
    }

    let mut password_prompt = Password::new(&prompt);

    if let Some((confirm_prompt, mismatch_message)) = &confirmation {
        password_prompt = password_prompt
            .with_custom_confirmation_message(confirm_prompt.as_str())
            .with_custom_confirmation_error_message(mismatch_message.as_str());
    } else {
        password_prompt = password_prompt.without_confirmation();
    }

    let value = password_prompt.prompt().map_err(inquire_error_to_pyx)?;

    if value.is_empty() {
        return Err(PyxError::Validation(empty_error));
    }

    Ok(value)
}

fn inquire_error_to_pyx(err: InquireError) -> PyxError {
    match err {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => PyxError::Cancelled,
        _ => PyxError::Validation(format!("Failed to read input: {err}")),
    }
}

/// Completion handler for provider names - matches Go's tap.Autocomplete behavior
#[derive(Clone)]
pub struct ProviderCompletion {
    providers: Vec<String>,
}

impl ProviderCompletion {
    pub fn new(providers: Vec<String>) -> Self {
        Self { providers }
    }
}

impl Autocomplete for ProviderCompletion {
    fn get_suggestions(
        &mut self,
        input: &str,
    ) -> std::result::Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        if input.is_empty() {
            return Ok(self.providers.clone());
        }

        let input_lower = input.to_lowercase();
        let filtered: Vec<String> = self
            .providers
            .iter()
            .filter(|provider| provider.to_lowercase().contains(&input_lower))
            .take(MAX_AUTOCOMPLETE_SUGGESTIONS)
            .cloned()
            .collect();

        Ok(filtered)
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted: Option<String>,
    ) -> std::result::Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(highlighted) = highlighted {
            return Ok(Some(highlighted));
        }

        if input.is_empty() {
            return Ok(None);
        }

        let input_lower = input.to_lowercase();
        let matches: Vec<String> = self
            .providers
            .iter()
            .filter(|provider| provider.to_lowercase().contains(&input_lower))
            .cloned()
            .collect();

        if matches.is_empty() {
            return Ok(None);
        }

        if matches.len() == 1 {
            return Ok(Some(matches[0].clone()));
        }

        // Find longest common prefix for partial completion
        let mut prefix = matches[0].clone();
        for provider in &matches[1..] {
            prefix = prefix
                .chars()
                .zip(provider.chars())
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| a)
                .collect();
        }

        if prefix.len() > input.len() {
            Ok(Some(prefix))
        } else {
            Ok(Some(input.to_string()))
        }
    }
}

/// Prompt user for provider selection with autocomplete.
/// Matches Go's PromptProvider behavior with tap.Autocomplete.
pub fn prompt_provider(providers: &[String]) -> Result<String> {
    let input = prompt_provider_input(providers)?;

    if input.is_empty() {
        return Err(PyxError::Cancelled);
    }

    resolve_provider_match(providers, &input)
}

fn prompt_provider_input(providers: &[String]) -> Result<String> {
    let completion = ProviderCompletion::new(providers.to_vec());

    Text::new("Select a provider")
        .with_autocomplete(completion)
        .prompt()
        .map_err(inquire_error_to_pyx)
}

/// Handles exact matches, substring matches, and prefix matches.
fn resolve_provider_match(providers: &[String], input: &str) -> Result<String> {
    if let Some(provider) = providers
        .iter()
        .find(|provider| provider.eq_ignore_ascii_case(input))
    {
        return Ok(provider.clone());
    }

    let input_lower = input.to_lowercase();
    let matches: Vec<&str> = providers
        .iter()
        .map(String::as_str)
        .filter(|provider| provider.to_lowercase().contains(&input_lower))
        .collect();

    if matches.is_empty() {
        return Err(PyxError::Validation(format!(
            "unknown provider '{input}'. Run 'pyx models update' to refresh"
        )));
    }

    if matches.len() == 1 {
        return Ok(matches[0].to_owned());
    }

    resolve_multiple_matches(&matches, &input_lower)
}

/// Resolve multiple matches by preferring prefix matches, then shortest.
fn resolve_multiple_matches(matches: &[&str], input_lower: &str) -> Result<String> {
    let prefix_matches: Vec<&str> = matches
        .iter()
        .copied()
        .filter(|provider| provider.to_lowercase().starts_with(input_lower))
        .collect();

    if prefix_matches.len() == 1 {
        return Ok(prefix_matches[0].to_owned());
    }

    let candidates = if prefix_matches.is_empty() {
        matches
    } else {
        &prefix_matches
    };

    candidates
        .iter()
        .copied()
        .min_by_key(|provider| provider.len())
        .map(str::to_owned)
        .ok_or_else(|| PyxError::Validation("no provider matches available".to_string()))
}

/// Prompt user for confirmation.
pub fn prompt_confirm(message: &str) -> Result<bool> {
    Confirm::new(message)
        .with_default(false)
        .prompt()
        .map_err(|e| PyxError::Validation(format!("Failed to read confirmation: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn providers(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn resolve_multiple_matches_prefers_shortest_when_no_prefix_match() {
        let matches = ["anthropic", "my-anthropic"];

        let resolved = resolve_multiple_matches(&matches, "ropic").expect("should resolve");

        assert_eq!(resolved, "anthropic");
    }

    #[test]
    fn resolve_multiple_matches_returns_error_when_empty() {
        let err = resolve_multiple_matches(&[], "openai").expect_err("empty matches should fail");
        assert!(matches!(err, PyxError::Validation(_)));
    }

    #[test]
    fn resolve_provider_match_prefers_prefix_then_shortest() {
        let providers = providers(&["anthropic", "my-anthropic", "meta"]);

        let resolved = resolve_provider_match(&providers, "an").expect("should resolve provider");

        assert_eq!(resolved, "anthropic");
    }
}
