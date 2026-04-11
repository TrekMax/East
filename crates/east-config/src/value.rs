use std::fmt;

/// A configuration value that can be stored in a config layer.
///
/// # Example
///
/// ```
/// use east_config::ConfigValue;
///
/// let s = ConfigValue::String("hello".into());
/// assert_eq!(s.as_str(), Some("hello"));
/// assert_eq!(format!("{s}"), "hello");
///
/// let n = ConfigValue::Integer(42);
/// assert_eq!(n.as_i64(), Some(42));
/// ```
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::module_name_repetitions)]
pub enum ConfigValue {
    /// A string value.
    String(std::string::String),
    /// A 64-bit integer value.
    Integer(i64),
    /// A floating-point value.
    Float(f64),
    /// A boolean value.
    Boolean(bool),
}

impl ConfigValue {
    /// Returns the value as a string slice, if it is a `String`.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as an `i64`, if it is an `Integer`.
    #[must_use]
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the value as an `f64`, if it is a `Float`.
    #[must_use]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns the value as a `bool`, if it is a `Boolean`.
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

impl fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(f, "{s}"),
            Self::Integer(i) => write!(f, "{i}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Boolean(b) => write!(f, "{b}"),
        }
    }
}
