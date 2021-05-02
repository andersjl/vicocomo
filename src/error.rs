//! The vicocomo error type.
//!

use crate::texts::get_text;

pub const SQLSTATE_FOREIGN_KEY_VIOLATION: &'static str = "23503";
pub const SQLSTATE_UNIQUE_VIOLATION: &'static str = "23505";

/// Vicocomo's error type.
///
/// The implementation of `Display` converts the error texts to a format
/// suitable as keys for [localization](../texts/index.html) and performs the
/// actual translation, see the [examples](#display-examples).
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// A model object cannot be deleted.
    ///
    CannotDelete(ModelError),

    /// A model object cannot be saved.
    ///
    CannotSave(ModelError),

    /// A database driver error.
    ///
    Database(DatabaseError),

    /// The input cannot be accepted.
    ///
    InvalidInput(String),

    #[doc(hidden)]
    None,

    /// Unspecified error.
    ///
    Other(String),

    /// Template engine error.
    ///
    Render(String),
}

impl Error {
    /// Create an `Error::Database`.
    ///
    pub fn database(sqlstate: Option<&str>, text: &str) -> Self {
        Self::Database(DatabaseError {
            sqlstate: sqlstate.map(|c| c.to_string()),
            text: text.to_string(),
        })
    }

    /// Create an `Error::InvalidInput`.
    ///
    pub fn invalid_input(text: &str) -> Self {
        Self::InvalidInput(text.to_string())
    }

    /// The variant is [`Database`](#variant.Database) and the database error
    /// code is `SQLSTATE 23503`.
    ///
    pub fn is_foreign_key_violation(&self) -> bool {
        match &self {
            Self::Database(DatabaseError { sqlstate, text: _ }) => {
                sqlstate.is_some()
                    && sqlstate.as_ref().unwrap()
                        == SQLSTATE_FOREIGN_KEY_VIOLATION
            }
            _ => false,
        }
    }

    /// The variant is [`Database`](#variant.Database) and the database error
    /// code is `SQLSTATE 23505`.
    ///
    pub fn is_unique_violation(&self) -> bool {
        match self {
            Self::Database(DatabaseError { sqlstate, text: _ }) => {
                sqlstate.is_some()
                    && sqlstate.as_ref().unwrap() == SQLSTATE_UNIQUE_VIOLATION
            }
            _ => false,
        }
    }

    #[doc(hidden)]
    pub fn nyi() -> Self {
        Self::other("NYI")
    }

    /// Create an `Error::Other`.
    ///
    pub fn other(text: &str) -> Self {
        Self::Other(text.to_string())
    }

    /// Create an `Error::Render`.
    ///
    pub fn render(text: &str) -> Self {
        Self::Render(text.to_string())
    }

    // --- private -----------------------------------------------------------

    fn format_cannot_variant(
        variant: &'static str,
        error: &ModelError,
    ) -> (String, String) {
        let kind = get_text(&Self::format_variant(variant), &[]);
        let mut text = vec![Self::format_model(variant, error)];
        text.extend(Self::format_fields(variant, error).drain(..));
        let text = text
            .iter()
            .map(|err| get_text(err, &[]))
            .collect::<Vec<_>>()
            .join("\n");
        (kind, text)
    }

    #[doc(hidden)] // used by the macro HtmlForm
    pub fn format_database(err: &DatabaseError) -> String {
        let DatabaseError { sqlstate, text } = err;
        let formatted = if let Some(ss) = sqlstate.as_ref() {
            format!("{}--", ss)
        } else {
            String::new()
        };
        Self::format_error("Database", &(formatted + text))
    }

    #[doc(hidden)] // used by the macro HtmlForm
    pub fn format_error(var: &'static str, err: &str) -> String {
        format!("error--{}--{}", var, err)
    }

    #[doc(hidden)] // used by the macro HtmlForm
    pub fn format_model(var: &'static str, err: &ModelError) -> String {
        let ModelError {
            model,
            general,
            field_errors: _,
        } = err;
        if let Some(ge) = general.as_ref() {
            format!("error--{}--{}--{}", var, model, ge)
        } else {
            Self::format_error(var, model)
        }
    }

    #[doc(hidden)] // used by the macro HtmlForm
    pub fn format_field(
        var: &'static str,
        mdl: &str,
        fld: &FieldError,
    ) -> Vec<String> {
        let FieldError { field, texts } = fld;
        texts
            .iter()
            .map(|err| format!("error--{}--{}--{}--{}", var, mdl, field, err))
            .collect()
    }

    fn format_fields(var: &'static str, err: &ModelError) -> Vec<String> {
        let ModelError {
            model,
            general: _,
            field_errors,
        } = err;
        let mut result = Vec::new();
        for fe in field_errors {
            result.extend(Self::format_field(var, model, fe).drain(..));
        }
        result
    }

    fn format_simple_variant(
        var: &'static str,
        text: &str,
    ) -> (String, String) {
        (
            get_text(&Self::format_variant(var), &[]),
            get_text(&Self::format_error(var, text), &[]),
        )
    }

    #[doc(hidden)] // used by the macro HtmlForm
    pub fn format_variant(var: &'static str) -> String {
        format!("error--{}", var)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cannot {
    (   $variant:ident,
        $name:literal,
        $mdl:literal,
        $mdl_err:expr,
    $(  $fld:literal: [ $( $fld_err:expr ),* ] ),*
    ) => {
        ::vicocomo::Error::$variant(::vicocomo::ModelError {
            model: $mdl.to_string(),
            general: if $mdl_err.trim().is_empty() {
                    None
                } else {
                    Some($mdl_err.to_string())
                },
            field_errors: vec![
            $(
                ::vicocomo::FieldError {
                    field: $fld.to_string(),
                    texts: vec![ $( $fld_err.to_string() ),* ],
                }
            ),*
            ],
        })
    };
}

/// Create an [`Error::CannotDelete`
/// ](error/enum.Error.html#variant.CannotDelete).
///
/// `mdl` is the name of the model as a string literal.
///
/// `mdl_err` is the general error message.  Only whitespace => `None`.
///
/// `field` is the name of a field as a string literal.
///
/// `field_err` are the error texts for the field.
///
/// The texts are expected to be [localized](../texts/index.html) before
/// showing them to an end user. `Error::to_string()` does that.
/// ```
/// use vicocomo::{cannot_delete, Error, FieldError, ModelError};
///
/// assert_eq!(
///     cannot_delete!("Mdl": "mdl-err", "f1": ["x", "y"], "f2": ["z"],),
///     Error::CannotDelete(ModelError {
///         model: "Mdl".to_string(),
///         general: Some("mdl-err".to_string()),
///         field_errors: vec![
///             FieldError {
///                 field: "f1".to_string(),
///                 texts: vec!["x".to_string(), "y".to_string()],
///             },
///             FieldError {
///                 field: "f2".to_string(),
///                 texts: vec!["z".to_string()],
///             },
///         ],
///     }),
/// );
/// ```
#[macro_export]
macro_rules! cannot_delete {
    (
        $mdl:literal: $mdl_err:expr,
    $(  $fld:literal: [ $( $fld_err:expr ),* $(  , )? ] ),*
    $(  , )?
    ) => {
        $crate::__cannot!(
            CannotDelete,
            "CannotDelete",
            $mdl,
            $mdl_err,
            $( $fld: [ $( $fld_err ),* ] ),*
        )
    };
}

/// Create an [`Error::CannotSave`
/// ](error/enum.Error.html#variant.CannotSave).
///
/// See [`cannot_delete!()`](macro.cannot_delete.html).
///
#[macro_export]
macro_rules! cannot_save {
    (
        $mdl:literal: $mdl_err:expr,
    $(  $fld:literal: [ $( $fld_err:expr ),* $(  , )? ] ),*
    $(  , )?
    ) => {
        $crate::__cannot!(
            CannotSave,
            "CannotSave",
            $mdl,
            $mdl_err,
            $( $fld: [ $( $fld_err ),* ] ),*
        )
    };
}

/// Simplify mapping another error type to this one.
///
#[macro_export]
macro_rules! map_error {
    ($variant: ident, $result: expr $( , )? ) => {
        ($result).map_err(|e| $crate::Error::$variant(e.to_string()))
    };
}

impl std::error::Error for Error {}

/// ### Display examples
/// ```
/// use vicocomo::{cannot_delete, cannot_save, Error, DatabaseError, FieldError, ModelError};
///
/// // All texts are localized, and to achieve unambigous translation
/// // "error--<variant>--" is prepended to get localization keys, so your
/// // localization configuration should look like below (only three of the
/// // examples have defined texts).
/// //
/// let _ = std::fs::write(
///     "config/texts.cfg",
///     "
///     \"error--CannotDelete\" => \"Cannot delete\",
///     \"error--CannotDelete--Model\" => \"Cannot delete Model\",
///     \"error--CannotDelete--Model--inconsistent\" => \"Cannot delete Model: Inconsistent\",
///     \"error--CannotDelete--Model--field1--row-1\" => \"field 1: First row\",
///     \"error--CannotDelete--Model--field1--error 666\" => \"DB error\",
///     \"error--CannotDelete--Model--field2--text-2\" => \"field 2: Second text\",
///     \"error--Database--some-db-error\" => \"Some DB error description\",
///     \"error--InvalidInput\" => \"Invalid input\",
///     \"error--InvalidInput--bad-input\" => \"Bad input\",
///     ",
/// );
///
/// assert_eq!(
///     cannot_delete!(
///         "Model": "inconsistent", "field1": ["row-1", "error 666"], "field2": ["text-2"],
///     ).to_string(),
///     "Cannot delete\nCannot delete Model: Inconsistent\nfield 1: First row\nDB error\nfield 2: Second text",
/// );
///
/// // CannotSave is just like CannotDelete.
/// // If general is only whitespace both the dashes and the text are omitted
/// // from the localization key. The following is without localization:
/// assert_eq!(
///     cannot_save!("Model": "\n ", "field1": ["text-1"], "field2": ["text-2"]).to_string(),
///     "error--CannotSave\
///     \nerror--CannotSave--Model\
///     \nerror--CannotSave--Model--field1--text-1\
///     \nerror--CannotSave--Model--field2--text-2",
/// );
///
/// // For Database, the sqlstate is prepended to the text before localization
/// assert_eq!(
///     Error::database(Some("12345"), "some-db-error").to_string(),
///     "error--Database\nerror--Database--12345--some-db-error",
/// );
/// // No sqlstate:
/// assert_eq!(
///     Error::database(None, "some-db-error").to_string(),
///     "error--Database\nSome DB error description",
/// );
///
/// // All the rest are alike, this one with localization:
/// assert_eq!(
///     Error::invalid_input("bad-input").to_string(),
///     "Invalid input\nBad input",
/// );
///
/// // This one has no localization:
/// assert_eq!(
///     Error::other("whatever").to_string(),
///     "error--Other\nerror--Other--whatever",
/// );
/// ```
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use ::v_htmlescape::escape;

        let (kind, text) = match self {
            Self::CannotDelete(error) => {
                Self::format_cannot_variant("CannotDelete", &error)
            }
            Self::CannotSave(error) => {
                Self::format_cannot_variant("CannotSave", &error)
            }
            Self::Database(error) => (
                get_text(&Self::format_variant("Database"), &[]),
                get_text(&Self::format_database(&error), &[]),
            ),
            Self::InvalidInput(text) => {
                Self::format_simple_variant("InvalidInput", &text)
            }
            Self::None => {
                (get_text(&Self::format_variant("None"), &[]), String::new())
            }
            Self::Other(text) => Self::format_simple_variant("Other", &text),
            Self::Render(text) => {
                Self::format_simple_variant("Render", &text)
            }
        };
        write!(f, "{}\n{}", escape(&kind), escape(&text))
    }
}

/// Create an `Error::Other`.
///
impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Self::Other(err.to_string())
    }
}

/// The contents of the [error](enum.Error.html)  variant [`Database`
/// ](enum.Error.html#variant.Database)
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatabaseError {
    /// Code according to the `SQLSTATE` standard.
    ///
    pub sqlstate: Option<String>,
    /// Error text as received from the database driver.
    ///
    pub text: String,
}

/// Describes an error preventing updating of a model field.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldError {
    /// The name of the model field.
    ///
    pub field: String,

    /// One or more descriptions of how the contents of the field causes the
    /// error.
    ///
    pub texts: Vec<String>,
}

/// The contents of the [error](enum.Error.html) variants [`CannotDelete`
/// ](enum.Error.html#variant.CannotDelete) and [`CannotSave`
/// ](enum.Error.html#variant.CannotSave)
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelError {
    /// The name of the model signalling the error.
    ///
    pub model: String,

    /// A description of the error that does not refer to any specific field.
    ///
    pub general: Option<String>,

    /// Field specific error descriptions.
    ///
    pub field_errors: Vec<FieldError>,
}
