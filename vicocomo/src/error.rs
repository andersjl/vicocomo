//! The vicocomo error type.
//!

use crate::texts::get_text;
use std::fmt::Display;

/// Grabbed from the [PosgreSQL docs
/// ](https://www.postgresql.org/docs/current/errcodes-appendix.html). The
/// actual SQLSTATE standard is not open source?
///
pub const SQLSTATE_FOREIGN_KEY_VIOLATION: &'static str = "23503";

/// Grabbed from the [PosgreSQL docs
/// ](https://www.postgresql.org/docs/current/errcodes-appendix.html). The
/// actual SQLSTATE standard is not open source?
///
pub const SQLSTATE_UNIQUE_VIOLATION: &'static str = "23505";

/// Vicocomo's error type.
///
/// The implementation of `Display` converts the error texts to a format
/// suitable as keys for [localization](../texts/index.html).
///
/// There is also a [`localize()`](#method.localize) method that also performs
/// the actual translation, see the [examples](#display-examples).
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// A database driver error.
    ///
    Database(DatabaseError),

    /// The input cannot be accepted.
    ///
    InvalidInput(String),

    /// An error referring to a model object.
    ///
    Model(ModelError),

    #[doc(hidden)]
    None,

    /// Unspecified error.
    ///
    Other(String),

    /// Template engine error.
    ///
    Render(String),

    /// An unexplainable bug, stop execution as graceful as possible.
    ///
    ThisCannotHappen(String),
}

impl Error {
    /// Create an `Error::Database`.
    ///
    pub fn database<T: Display>(sqlstate: Option<&str>, text: T) -> Self {
        Self::Database(DatabaseError {
            sqlstate: sqlstate.map(|c| c.to_string()),
            text: text.to_string(),
        })
    }

    /// Create an `Error::InvalidInput`.
    ///
    pub fn invalid_input<T: Display>(text: T) -> Self {
        Self::InvalidInput(text.to_string())
    }

    /// The variant is [`Database`](#variant.Database) and the database error
    /// code is `sqlstate`.
    ///
    pub fn is_database_error(&self, sqlstate: &str) -> bool {
        match self {
            Error::Database(de) => {
                de.sqlstate.as_ref().map(|s| s == sqlstate).unwrap_or(false)
            }
            _ => false,
        }
    }

    /// The variant is [`Database`](#variant.Database) and the database error
    /// code is `SQLSTATE 23503`.
    ///
    pub fn is_foreign_key_violation(&self) -> bool {
        self.is_database_error(SQLSTATE_FOREIGN_KEY_VIOLATION)
    }

    /// The variant is [`Database`](#variant.Database) and the database error
    /// code is `SQLSTATE 23505`.
    ///
    pub fn is_unique_violation(&self) -> bool {
        self.is_database_error(SQLSTATE_UNIQUE_VIOLATION)
    }

    /// Like [`to_string()`](#display-examples), but [localizes
    /// ](../texts/index.html) the texts before collecting them in a string.
    ///
    /// Unlike the [`t!`](../macro.t.html) macro, `localize()` cannot use
    /// parameterized substitution.
    ///
    /// ## examples
    /// ```
    /// use vicocomo::{model_error, Error};
    /// std::fs::create_dir_all("config").unwrap();
    ///
    /// // See the implementation of [`Display`](#display-examples) for
    /// // formatting before localization.
    /// //
    /// // Suppose a localization configuration like below (only three of the
    /// // examples have defined texts.
    /// //
    /// let _ = std::fs::write(
    ///     "config/texts.cfg",
    ///     "
    ///     \"error--Model-CannotDelete\" => \"Cannot delete\",
    ///     \"error--Model-CannotDelete--Model\" => \"Cannot delete Model\",
    ///     \"error--Model-CannotDelete--Model--assoc--some-error\" => \"some assoc error\",
    ///     \"error--Model-CannotDelete--Model--field1--row-1\" => \"field 1: First row\",
    ///     \"error--Model-CannotDelete--Model--field1--error 666\" => \"DB error\",
    ///     \"error--Model-CannotDelete--Model--field2\" => \"field 2 error\",
    ///     \"error--Model-CannotDelete--Model--inconsistent\" => \"Cannot delete Model: Inconsistent\",
    ///     \"error--Model-CannotSave--Model--field1--omitted\" => \"\",
    ///     \"error--Database--some-db-error\" => \"Some DB error description\",
    ///     \"error--InvalidInput\" => \"Invalid input\",
    ///     \"error--InvalidInput--bad-input\" => \"Bad input\",
    ///     ",
    /// );
    /// vicocomo::texts::initialize(None);
    ///
    /// assert_eq!(
    ///     model_error!(
    ///         CannotDelete,
    ///         "Model": "inconsistent",
    ///         "field1": ["row-1", "error 666"], "field2": [],
    ///         assoc "assoc": ["some-error"],
    ///     ).localize(),
    ///     "Cannot delete\
    ///     \nCannot delete Model: Inconsistent\
    ///     \nfield 1: First row\
    ///     \nDB error\
    ///     \nfield 2 error\
    ///     \nsome assoc error",
    /// );
    ///
    /// // If general is only whitespace it is omitted.
    /// // Without definitions in `texts.cfg` the return value is identical to
    /// // `to_string()`.
    /// // If the definition in `texts.cfg` is an empty string, that line is
    /// // omitted.
    /// //
    /// assert_eq!(
    ///     model_error!(
    ///         CannotSave,
    ///         "Model": "\n ",
    ///         "field1": ["text-1", "omitted"], "field2": []
    ///     ).localize(),
    ///     "error--Model-CannotSave\
    ///     \nerror--Model-CannotSave--Model\
    ///     \nerror--Model-CannotSave--Model--field1--text-1\
    ///     \nerror--Model-CannotSave--Model--field2",
    /// );
    ///
    /// // Database without sqlstate:
    /// assert_eq!(
    ///     Error::database(None, "some-db-error").localize(),
    ///     "error--Database\nSome DB error description",
    /// );
    ///
    /// // All the rest are alike:
    /// assert_eq!(
    ///     Error::invalid_input("bad-input").localize(),
    ///     "Invalid input\nBad input",
    /// );
    /// ```
    pub fn localize(&self) -> String {
        self.format(true).join("\n")
    }

    #[doc(hidden)]
    pub fn nyi() -> Self {
        Self::other("NYI")
    }

    /// Returne the *second* line of the formatted error, and localize it if
    /// `localize`.
    ///
    pub fn one_liner(&self, localize: bool) -> String {
        let lines = self.format(localize);
        match lines.len() {
            0 => String::new(),
            1 => lines[0].clone(),
            _ => lines[1].clone(),
        }
    }

    /// Create an `Error::Other`.
    ///
    pub fn other<T: Display>(text: T) -> Self {
        Self::Other(text.to_string())
    }

    /// Create an `Error::Render`.
    ///
    pub fn render<T: Display>(text: T) -> Self {
        Self::Render(text.to_string())
    }

    /// Create an `Error::ThisCannotHappen`.
    ///
    pub fn this_cannot_happen<T: Display>(text: T) -> Self {
        Self::ThisCannotHappen(text.to_string())
    }

    /// Like [`to_string()`](#display-examples), but if the formatted error
    /// has multiple lines they are not joined.
    ///
    pub fn to_strings(&self) -> Vec<String> {
        self.format(false)
    }

    // --- private -----------------------------------------------------------

    fn format_assocs(err: &ModelError) -> Vec<String> {
        let mut result = Vec::new();
        let variant_kind = Self::format_model_kind(err);
        for ae in &err.assoc_errors {
            result.extend(
                Self::format_field(&variant_kind, &err.model, ae).drain(..),
            );
        }
        result
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
    pub fn format_error(var: &str, err: &str) -> String {
        format!("error--{}--{}", var, err)
    }

    #[doc(hidden)] // used by the macro HtmlForm
    pub fn format_field(
        var: &str,
        mdl: &str,
        fld: &(String, Vec<String>),
    ) -> Vec<String> {
        if fld.1.is_empty() {
            vec![format!("error--{}--{}--{}", var, mdl, fld.0)]
        } else {
            fld.1
                .iter()
                .map(|e| format!("error--{}--{}--{}--{}", var, mdl, fld.0, e))
                .collect()
        }
    }

    fn format_fields(err: &ModelError) -> Vec<String> {
        let mut result = Vec::new();
        let variant_kind = Self::format_model_kind(err);
        for fe in &err.field_errors {
            result.extend(
                Self::format_field(&variant_kind, &err.model, fe).drain(..),
            );
        }
        result
    }

    #[doc(hidden)] // used by the macro HtmlForm
    pub fn format_model(err: &ModelError) -> String {
        let variant_kind = Self::format_model_kind(err);
        if let Some(ge) = err.general.as_ref() {
            format!("error--{}--{}--{}", &variant_kind, err.model, ge)
        } else {
            Self::format_error(&variant_kind, &err.model)
        }
    }

    #[doc(hidden)] // used by the macro HtmlForm
    pub fn format_model_kind(err: &ModelError) -> String {
        format!(
            "Model-{}",
            match err.error {
                ModelErrorKind::CannotSave => "CannotSave",
                ModelErrorKind::CannotDelete => "CannotDelete",
                ModelErrorKind::Invalid => "Invalid",
                ModelErrorKind::NotFound => "NotFound",
                ModelErrorKind::NotUnique => "NotUnique",
            }
        )
    }

    fn format_simple_variant(var: &'static str, text: &str) -> Vec<String> {
        vec![Self::format_variant(var), Self::format_error(var, text)]
    }

    #[doc(hidden)] // used by the macro HtmlForm
    pub fn format_variant(var: &str) -> String {
        format!("error--{}", var)
    }

    fn format(&self, localize: bool) -> Vec<String> {
        use v_htmlescape::escape;

        let mut texts = match self {
            Self::Database(error) => vec![
                Self::format_variant("Database"),
                Self::format_database(&error),
            ],
            Self::InvalidInput(text) => {
                Self::format_simple_variant("InvalidInput", &text)
            }
            Self::Model(error) => {
                let mut texts = vec![
                    Self::format_variant(&Self::format_model_kind(error)),
                    Self::format_model(error),
                ];
                texts.extend(Self::format_fields(error).drain(..));
                texts.extend(Self::format_assocs(error).drain(..));
                texts
            }
            Self::None => vec![Self::format_variant("None")],
            Self::Other(text) => Self::format_simple_variant("Other", &text),
            Self::Render(text) => {
                Self::format_simple_variant("Render", &text)
            }
            Self::ThisCannotHappen(text) => {
                Self::format_simple_variant("ThisCannotHappen", &text)
            }
        };
        let result = texts.drain(..).map(|err| {
            escape(&if localize { get_text(&err, &[]) } else { err })
                .to_string()
        });
        if localize {
            result.filter(|s| !s.is_empty()).collect()
        } else {
            result.collect()
        }
    }
}

/// Check for an error variant, optionally with specific content.
/// ```
/// use vicocomo::{is_error, model_error, Error, ModelError, ModelErrorKind};
///
/// assert!(is_error!(
///     model_error!(CannotDelete, "Mdl": "mdl-err", "f1": ["x", "y"], "f2": ["z"], assoc "a": ["u"]),
///     Model
/// ));
///
/// assert!(is_error!(
///     model_error!(CannotDelete, "Mdl": "mdl-err", "f1": ["x", "y"], "f2": ["z"], assoc "a": ["u"]),
///     Model(CannotDelete),
/// ));
///
/// assert!(is_error!(
///     model_error!(CannotDelete, "Mdl": "mdl-err", "f1": ["x", "y"], "f2": ["z"], assoc "a": ["u"]),
///     Model(CannotDelete, "Mdl", Some("mdl-err".to_string())),
/// ));
///
/// assert!(is_error!(
///     model_error!(CannotDelete, "Mdl": "mdl-err", "f1": ["x", "y"], "f2": ["z"], assoc "a": ["u"]),
///     Model(CannotDelete, "Mdl", Some("mdl-err".to_string()), "f2", []),
/// ));
///
/// assert!(is_error!(
///     model_error!(CannotDelete, "Mdl": "mdl-err", "f1": ["x", "y"], "f2": ["z"], assoc "a": ["u"]),
///     Model(CannotDelete, "Mdl", Some("mdl-err".to_string()), "f2", ["z"]),
/// ));
///
/// assert!(is_error!(
///     model_error!(CannotDelete, "Mdl": "mdl-err", "f1": ["x", "y"], "f2": ["z"], assoc "a": ["u"]),
///     Model(CannotDelete, "Mdl", Some("mdl-err".to_string()), "f1", ["y"]),
/// ));
///
/// assert!(is_error!(
///     model_error!(CannotDelete, "Mdl": "mdl-err", "f1": ["x", "y"], "f2": ["z"], assoc "a": ["u"]),
///     Model(CannotDelete, "Mdl", Some("mdl-err".to_string()), "a", ["u"],
///     ),
/// ));
///
/// assert!(is_error!(
///     model_error!(CannotDelete, "Mdl": "mdl-err", "f1": ["x", "y"], "f2": ["z"], assoc "a": ["u"]),
///     Model(CannotDelete, "Mdl", Some("mdl-err".to_string()), "f1", ["x", "y"], "a", ["u"], "f2", ["z"]),
/// ));
///
/// ```
#[macro_export]
macro_rules! is_error {
    (   $error:expr, $variant:ident $( , )?
    ) => {
        match $error {
            vicocomo::Error::$variant(_) => true,
            _ => false,
        }
    };
    (   $error:expr, Database ( $state:expr $( , $text:expr )? $( , )? ) $( , )?
    ) => {
        match $error {
            vicocomo::Error::Database(de) => {
                de.sqlstate == $state $( && de.text == $text )?
            }
            _ => false,
        }
    };
    (   $error:expr,
        Model (
            $kind:ident
        $(  , $model:expr, $general_error:expr
            $( , $field:expr, [ $( $fld_error:expr ),* ] )*
        )?
        $(  , )?
        ) $( , )?
    ) => {
        match $error {
            vicocomo::Error::Model(me) => {
                me.error == vicocomo::ModelErrorKind::$kind
            $(  && me.model == $model
                && me.general == $general_error
             $(
                && (
                    vicocomo::ModelError::fld_errors_include(
                        &me.field_errors,
                        $field,
                        &[ $( $fld_error ),* ],
                    )
                    || vicocomo::ModelError::fld_errors_include(
                        &me.assoc_errors,
                        $field,
                        &[ $( $fld_error ),* ],
                    )
                )
             )*
            )?
            }
            _ => false,
        }
    };
    (   $error:expr, $variant:ident ( $text:expr ) $( , )? ) => {
        match $error {
            vicocomo::Error::$variant(t) => t == $text,
            _ => false,
        }
    };
}

/// Create an [`Error::Model`](error/enum.Error.html#variant.Model).
///
/// `$error_kind` is the [`ModelErrorKind`](enum.ModelErrorKind.html).
///
/// `$model` is the name of the model as a string literal.
///
/// `$general_error` is the general error message.  Only whitespace => `None`.
///
/// `$field` is the name of a field with problems as a string literal.
///
/// `$fld_error` are the error texts for the field.  May be empty.
///
/// `$assoc` is the name of a [has-many
/// ](derive.ActiveRecord.html#vicocomo_has_many--) association as a string
/// literal.
///
/// `$ass_error` are the error texts for the association.  May be empty.
///
/// The texts are expected to be [localized](../texts/index.html) before
/// showing them to an end user. `Error::to_string()` does that.
/// ```
/// use vicocomo::{model_error, Error, ModelError, ModelErrorKind};
///
/// assert_eq!(
///     model_error!(CannotDelete, "Mdl": "mdl-err", "f1": ["x", "y"], "f2": [], assoc "a": ["z"]),
///     Error::Model(ModelError {
///         error: ModelErrorKind::CannotDelete,
///         model: "Mdl".to_string(),
///         general: Some("mdl-err".to_string()),
///         field_errors: vec![
///             ("f1".to_string(), vec!["x".to_string(), "y".to_string()]),
///             ("f2".to_string(), vec![]),
///         ],
///         assoc_errors: vec![("a".to_string(), vec!["z".to_string()])],
///     }),
/// );
/// ```
#[macro_export]
macro_rules! model_error {
    (   $error_kind:ident,
        $model:literal: $general_error:expr
    $(  , $field:literal: [ $( $fld_error:expr ),* ] )*
    $(  , assoc $assoc:literal: [ $( $ass_error:expr ),* ] )*
    $(  , )?
    ) => {
        {
            vicocomo::Error::Model(vicocomo::ModelError {
                error: vicocomo::ModelErrorKind::$error_kind,
                model: $model.to_string(),
                general: if $general_error.trim().is_empty() {
                        None
                    } else {
                        Some($general_error.to_string())
                    },
                field_errors: vec![
                $(  ($field.to_string(), vec![$($fld_error.to_string()),*]) ),*
                ],
                assoc_errors: vec![
                $(  ($assoc.to_string(), vec![$($ass_error.to_string()),*]) ),*
                ],
            })
        }
    };
}

/// Simplify mapping another error type to this one.
///
/// Requires `$variant` to be one of `InvalidInput`, `Other`, `Render`, ot
/// `ThisCannotHappen`.
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
/// use vicocomo::{model_error, Error};
///
/// // To facilitate unambigous translation, "error--<variant>--" is prepended
/// // to the field errors and to the  general error (only if present) to get
/// // localization keys.
/// //
/// assert_eq!(
///     model_error!(
///         CannotDelete,
///         "Model": "inconsistent",
///         "field1": ["row-1", "error 666"], "field2": ["text-2"],
///     ).to_string(),
///     "error--Model-CannotDelete\
///     \nerror--Model-CannotDelete--Model--inconsistent\
///     \nerror--Model-CannotDelete--Model--field1--row-1\
///     \nerror--Model-CannotDelete--Model--field1--error 666\
///     \nerror--Model-CannotDelete--Model--field2--text-2",
/// );
///
/// // If general is only whitespace it is omitted.
/// // An empty field error text array still generates one empty field error.
/// assert_eq!(
///     model_error!(
///         CannotSave,
///         "Model": "\n ",
///         "field1": ["text-1"], "field2": []
///     ).to_string(),
///     "error--Model-CannotSave\
///     \nerror--Model-CannotSave--Model\
///     \nerror--Model-CannotSave--Model--field1--text-1\
///     \nerror--Model-CannotSave--Model--field2",
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
///     "error--Database\nerror--Database--some-db-error",
/// );
///
/// // All the rest are alike:
/// assert_eq!(
///     Error::other("whatever").to_string(),
///     "error--Other\nerror--Other--whatever",
/// );
/// ```
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.format(false).join("\n"))
    }
}

/// Create an `Error::Other`.
///
impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Self::Other(err.to_string())
    }
}

/// Create an `Error::Other`.
///
impl From<String> for Error {
    fn from(err: String) -> Self {
        Self::Other(err)
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

/// The contents of the [error](enum.Error.html) variant [`Model`
/// ](enum.Error.html#variant.Model).
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelError {
    /// The error kind.
    ///
    pub error: ModelErrorKind,

    /// The name of the model signalling the error.
    ///
    pub model: String,

    /// A description of the error that does not refer to any specific field.
    ///
    pub general: Option<String>,

    /// Field specific error descriptions.
    ///
    /// `(`*field name*`, `*error texts*`)`.
    ///
    pub field_errors: Vec<(String, Vec<String>)>,

    /// Errors referring to [has-many
    /// ](../derive.ActiveRecord.html#vicocomo_has_many--) associations.
    ///
    /// `(`*association name*`, `*error texts*`)`.
    ///
    pub assoc_errors: Vec<(String, Vec<String>)>,
}

impl ModelError {
    #[doc(hidden)] // used by the macro is_error
    pub fn fld_errors_include(
        errs: &[(String, Vec<String>)],
        fld: &str,
        txts: &[&str],
    ) -> bool {
        errs.iter().any(|e| {
            e.0 == fld && !txts.iter().any(|t| !e.1.iter().any(|e| e == t))
        })
    }
}

/// The error kind, the names are self-explanatory.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelErrorKind {
    CannotSave,
    CannotDelete,
    Invalid,
    NotFound,
    NotUnique,
}
