//! Structs implementing a simple flash mechanism for web applications.
//!

use crate::HttpServerIf;
use ::serde::{Deserialize, Serialize};
use ::std::collections::HashMap;

/// A flash object storing and reading notifications and alerts in an
/// [HTTP server session](struct.HttpServerIf.html).
///
/// A flash is forgotten when read, see [`get()`](#method.get).
///
pub struct Flash<'srv, 'req> {
    server: HttpServerIf<'srv, 'req>,
    // { severity => [message, ... ], ... }
    flashes: HashMap<String, Vec<String>>,
}

const FLASH_KEY: &'static str = "__vicocomo__FlashObj";

impl<'srv, 'req> Flash<'srv, 'req> {
    /// Initialize the flashes from the `srv` session and mark as not shown.
    ///
    pub fn new(srv: HttpServerIf<'srv, 'req>) -> Self {
        Self {
            server: srv,
            flashes: srv
                .session_get(FLASH_KEY)
                .unwrap_or_else(|| HashMap::new()),
        }
    }

    /// Clear the flashes, also from the session.
    ///
    pub fn clear(&mut self) {
        self.flashes.clear();
        self.store();
    }

    /// Get a copy of the flash messages and return them as [`FlashData`
    /// ](struct.FlashData.html). See [`remove()`](#method.remove).
    ///
    pub fn peek(&self, severities: &[&str]) -> Vec<FlashData> {
        let mut result = Vec::new();
        for severity in severities {
            match self.flashes.get(*severity) {
                Some(messages) => {
                    result.extend(messages.iter().map(|message| FlashData {
                        severity: severity.to_string(),
                        message: message.to_string(),
                    }));
                }
                None => (),
            }
        }
        result
    }

    /// Add a new flash message to those stored under `severity` and update
    /// the session.
    ///
    /// If the exact message is already `push()`-ed, do nothing and return
    /// `false`.
    ///
    /// Otherwise push it and return `true`
    ///
    /// `severity` and `message` must not contain HTML markup.
    ///
    pub fn push(&mut self, severity: &str, message: &str) -> bool {
        let msg = message.to_string();
        match self.flashes.get_mut(severity) {
            Some(msgs) => {
                if msgs.iter().any(|m| m == message) {
                    return false;
                }
                msgs.push(msg);
            }
            None => {
                self.flashes.insert(severity.to_string(), vec![msg]);
            }
        }
        self.store();
        true
    }

    /// Remove and forget (also in the session) flash messages and return them
    /// as [`FlashData`](struct.FlashData.html).
    ///
    /// Only the `severities` severities are removed.
    ///
    /// The returned pairs are ordered first after the `severities`, then in
    /// the order they were [`push()`ed](method.push.html).
    ///
    /// `"<br>"` is substituted for `"\n"` in the [`message`
    /// ](struct.FlashData.html#field.message) field.
    ///
    pub fn remove(&mut self, severities: &[&str]) -> Vec<FlashData> {
        let mut removed = Vec::new();
        for severity in severities {
            match self.flashes.get_mut(*severity) {
                Some(messages) => {
                    removed.extend(messages.drain(..).map(|message| {
                        FlashData {
                            severity: severity.to_string(),
                            message: message.replace("\n", "<br>"),
                        }
                    }))
                }
                None => (),
            }
        }
        self.store();
        removed
    }

    // store the flashes in the session
    fn store(&self) {
        let _ = self.server.session_set(FLASH_KEY, &self.flashes);
    }
}

/// The type returned by [`Flash::peek()`](struct.Flash.html#method.peek) and
/// [`Flash::remove()`](struct.Flash.html#method.remove).
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlashData {
    /// The message severity as an HTML safe string.
    pub severity: String,
    /// The message text as an HTML safe string that may contain `<br>`.
    pub message: String,
}
