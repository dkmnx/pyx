//! Shared interactive prompt helpers.

use crate::error::{PyxError, Result};
use dialoguer::{Confirm, FuzzySelect, Password, theme::ColorfulTheme};

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

    let theme = ColorfulTheme::default();
    let mut password_prompt = Password::with_theme(&theme).with_prompt(prompt);

    if let Some((confirm_prompt, mismatch_message)) = confirmation {
        password_prompt = password_prompt.with_confirmation(confirm_prompt, mismatch_message);
    }

    let value = password_prompt
        .interact()
        .map_err(|e| PyxError::Validation(format!("Failed to read input: {}", e)))?;

    if !allow_empty && value.is_empty() {
        return Err(PyxError::Validation(empty_error));
    }

    Ok(value)
}

/// Prompt user to select a provider from a list with fuzzy search.
pub fn prompt_provider(providers: &[String]) -> Result<String> {
    let theme = ColorfulTheme::default();

    let selection = FuzzySelect::with_theme(&theme)
        .with_prompt("Select a provider")
        .items(providers)
        .default(0)
        .interact()
        .map_err(|e| PyxError::Validation(format!("Failed to select provider: {}", e)))?;

    Ok(providers[selection].clone())
}

/// Prompt user for confirmation.
pub fn prompt_confirm(message: &str) -> Result<bool> {
    let theme = ColorfulTheme::default();

    Confirm::with_theme(&theme)
        .with_prompt(message)
        .default(false)
        .interact()
        .map_err(|e| PyxError::Validation(format!("Failed to read confirmation: {}", e)))
}
