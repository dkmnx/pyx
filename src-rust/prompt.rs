//! Shared interactive prompt helpers.

use crate::error::{PyxError, Result};
use dialoguer::{Password, theme::ColorfulTheme};

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
