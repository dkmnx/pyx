//! Shared interactive prompt helpers.

use crate::error::{PyxError, Result};
use inquire::{Autocomplete, Confirm, InquireError, Password, Text};

pub struct SecretPromptOptions {
    pub prompt: String,
    pub helper: Option<String>,
    pub confirmation: Option<(String, String)>,
    pub empty_error: String,
    pub allow_empty: bool,
}

pub fn prompt_secret(options: SecretPromptOptions) -> Result<String> {
    let SecretPromptOptions {
        prompt,
        helper,
        confirmation,
        empty_error,
        allow_empty,
    } = options;

    if let Some(helper_text) = helper {
        println!("{}", helper_text);
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

    if allow_empty {
        password_prompt = password_prompt.without_confirmation();
    }

    let value = password_prompt.prompt().map_err(inquire_error_to_pyx)?;

    if !allow_empty && value.is_empty() {
        return Err(PyxError::Validation(empty_error));
    }

    Ok(value)
}

fn inquire_error_to_pyx(err: InquireError) -> PyxError {
    match err {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            PyxError::Validation("operation cancelled".to_string())
        }
        _ => PyxError::Validation(format!("Failed to read input: {}", err)),
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
            .take(7) // Max 7 results like Go
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
    let completion = ProviderCompletion::new(providers.to_vec());

    let input = Text::new("Select a provider")
        .with_autocomplete(completion)
        .prompt()
        .map_err(inquire_error_to_pyx)?;

    // Empty input means cancelled (Ctrl+C)
    if input.is_empty() {
        return Err(PyxError::Validation("operation cancelled".to_string()));
    }

    // Find the selected provider (case-insensitive match)
    for provider in providers {
        if provider.eq_ignore_ascii_case(&input) {
            return Ok(provider.clone());
        }
    }

    // Filter matches like Go does
    let input_lower = input.to_lowercase();
    let matches: Vec<String> = providers
        .iter()
        .filter(|p| p.to_lowercase().contains(&input_lower))
        .cloned()
        .collect();

    if matches.is_empty() {
        return Err(PyxError::Validation(format!(
            "unknown provider '{}'. Run 'pyx models update' to refresh",
            input
        )));
    }

    if matches.len() == 1 {
        return Ok(matches[0].clone());
    }

    // Multiple matches - prefer prefix match, then shortest
    let prefix_matches: Vec<String> = matches
        .iter()
        .filter(|p| p.to_lowercase().starts_with(&input_lower))
        .cloned()
        .collect();

    if prefix_matches.len() == 1 {
        return Ok(prefix_matches[0].clone());
    }

    if !prefix_matches.is_empty() {
        // Multiple prefix matches - pick shortest
        return Ok(prefix_matches.into_iter().min_by_key(|p| p.len()).unwrap());
    }

    // No prefix matches - pick shortest substring match
    Ok(matches.into_iter().min_by_key(|p| p.len()).unwrap())
}

/// Prompt user for confirmation.
pub fn prompt_confirm(message: &str) -> Result<bool> {
    Confirm::new(message)
        .with_default(false)
        .prompt()
        .map_err(|e| PyxError::Validation(format!("Failed to read confirmation: {}", e)))
}
