use std::collections::BTreeMap;

use crate::error::TemplateError;

/// A simple string-substitution template engine.
///
/// Supports `${key}` syntax only. No filters, conditionals, or loops.
/// `$${...}` produces a literal `${...}`.
///
/// # Example
///
/// ```
/// use std::collections::BTreeMap;
/// use east_command::template::TemplateEngine;
///
/// let engine = TemplateEngine::new();
/// let mut vars = BTreeMap::new();
/// vars.insert("workspace.root".to_string(), "/my/ws".to_string());
/// let result = engine.render("root: ${workspace.root}", &vars, "test").unwrap();
/// assert_eq!(result, "root: /my/ws");
/// ```
#[allow(clippy::module_name_repetitions)]
pub struct TemplateEngine;

impl TemplateEngine {
    /// Create a new template engine.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Render a template string by substituting `${key}` references.
    ///
    /// # Errors
    ///
    /// - [`TemplateError::MissingKey`] if a referenced key is not in `vars`.
    /// - [`TemplateError::UnterminatedVariable`] if `${` has no matching `}`.
    pub fn render(
        &self,
        input: &str,
        vars: &BTreeMap<String, String>,
        source_hint: &str,
    ) -> Result<String, TemplateError> {
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' {
                match chars.peek() {
                    Some('$') => {
                        // Check if this is $${ escape
                        chars.next(); // consume second $
                        if chars.peek() == Some(&'{') {
                            chars.next(); // consume {
                                          // Find the closing } and output literally as ${...}
                            result.push_str("${");
                            loop {
                                match chars.next() {
                                    Some('}') => {
                                        result.push('}');
                                        break;
                                    }
                                    Some(c) => result.push(c),
                                    None => {
                                        // Unterminated but it was an escape, just output what we have
                                        break;
                                    }
                                }
                            }
                        } else {
                            // Just two dollars, not followed by {
                            result.push('$');
                            result.push('$');
                        }
                    }
                    Some('{') => {
                        chars.next(); // consume {
                        let mut key = String::new();
                        loop {
                            match chars.next() {
                                Some('}') => break,
                                Some(c) => key.push(c),
                                None => {
                                    return Err(TemplateError::UnterminatedVariable {
                                        source_hint: source_hint.to_string(),
                                    });
                                }
                            }
                        }
                        let value = vars.get(&key).ok_or_else(|| TemplateError::MissingKey {
                            key: key.clone(),
                            source_hint: source_hint.to_string(),
                        })?;
                        result.push_str(value);
                    }
                    _ => {
                        // Lone $, not followed by { or $
                        result.push('$');
                    }
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}
