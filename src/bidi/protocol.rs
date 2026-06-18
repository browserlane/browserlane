use std::sync::atomic::{AtomicI64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Atomic counter for generating unique command IDs.
static COMMAND_ID: AtomicI64 = AtomicI64::new(0);

/// Returns the next unique command ID.
pub fn next_id() -> i64 {
    COMMAND_ID.fetch_add(1, Ordering::SeqCst) + 1
}

/// A BiDi command to be sent to the browser.
#[derive(Debug, Serialize)]
pub struct Command {
    pub id: i64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A BiDi response from the browser.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Response {
    #[serde(default, skip_serializing_if = "is_zero_id")]
    pub id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorData>,
}

fn is_zero_id(id: &i64) -> bool {
    *id == 0
}

/// An error in a BiDi response.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub message: String,
}

/// A BiDi event from the browser.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Event {
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A generic BiDi message that can be either a response or an event.
#[derive(Debug, Default, Deserialize)]
pub struct Message {
    // Response fields
    pub id: Option<i64>,
    pub result: Option<Value>,
    pub error: Option<Value>,

    // Event fields
    #[serde(default)]
    pub method: String,
    pub params: Option<Value>,
}

impl Message {
    /// Returns true if the message is a response (has an ID).
    pub fn is_response(&self) -> bool {
        self.id.is_some()
    }

    /// Returns true if the message is an event (has a method but no ID).
    pub fn is_event(&self) -> bool {
        !self.method.is_empty() && self.id.is_none()
    }

    /// Returns true if the message is an error response.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Parses the error field and returns an ErrorData.
    pub fn get_error(&self) -> Result<Option<ErrorData>, serde_json::Error> {
        let raw = match &self.error {
            None => return Ok(None),
            Some(v) => v,
        };

        // Try to deserialize as an ErrorData object.
        if let Ok(err_data) = serde_json::from_value::<ErrorData>(raw.clone()) {
            return Ok(Some(err_data));
        }

        // Otherwise it might be a plain string.
        let err_str: String = serde_json::from_value(raw.clone())?;
        Ok(Some(ErrorData {
            error: err_str.clone(),
            message: err_str,
        }))
    }
}

impl Command {
    /// Serializes the command to JSON.
    pub fn marshal(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

/// Creates a new BiDi command with a unique ID.
pub fn new_command(method: &str, params: Option<Value>) -> Command {
    Command {
        id: next_id(),
        method: method.to_string(),
        params,
    }
}

/// Parses a JSON message into a Message struct.
pub fn unmarshal_message(data: &[u8]) -> Result<Message, serde_json::Error> {
    serde_json::from_slice(data)
}
