use crate::domain::model::language::Language;
use crate::infrastructure::database::pool::InfrastructureError;
use std::collections::HashMap;

/// Loads translation strings for the given language.
///
/// # Parameters
///
/// - `language`: The language for which to load translations.
///
/// # Returns
///
/// A `HashMap` mapping translation keys to their corresponding localized strings.
///
/// # Errors
///
/// Returns an [`InfrastructureError`] if:
/// - The JSON translation file for the requested language cannot be deserialized.
pub fn load_translations(
    language: &Language,
) -> Result<HashMap<String, String>, InfrastructureError> {
    let data = match language {
        Language::English => include_str!("../../../translations/en.json"),
        Language::French => include_str!("../../../translations/fr.json"),
    };
    Ok(serde_json::from_str(data)?)
}
