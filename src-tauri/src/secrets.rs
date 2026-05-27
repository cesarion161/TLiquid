//! Secure secret storage backed by the macOS Keychain (P0-005).
//!
//! API keys are stored per provider id and never written to the plaintext
//! settings file or logged (PRD FR-050, FR-051, FR-052). Nothing in this module
//! logs, and the only data placed in an error is the Keychain backend's own
//! message — never the key value (see [`map_get`]/[`map_delete`] and the tests).

use crate::error::{AppError, Result};

/// Keychain service name. Entries are keyed by provider id (the "account").
const SERVICE: &str = "com.tliquid.app";

fn entry(provider_id: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, provider_id).map_err(|e| AppError::Secret(e.to_string()))
}

pub fn set_key(provider_id: &str, api_key: &str) -> Result<()> {
    map_set(entry(provider_id)?.set_password(api_key))
}

pub fn get_key(provider_id: &str) -> Result<Option<String>> {
    map_get(entry(provider_id)?.get_password())
}

pub fn delete_key(provider_id: &str) -> Result<()> {
    map_delete(entry(provider_id)?.delete_credential())
}

/// Map a Keychain write. The error carries only the backend's own message —
/// never the key value being written (FR-051/FR-052).
fn map_set(result: keyring::Result<()>) -> Result<()> {
    result.map_err(|e| AppError::Secret(e.to_string()))
}

/// Map a Keychain read into "maybe a key": a missing entry is `None`, not an
/// error, so "no key configured" is a normal state callers can branch on. Any
/// other backend error is surfaced as [`AppError::Secret`] (its message comes
/// from the Keychain, never the key value).
fn map_get(result: keyring::Result<String>) -> Result<Option<String>> {
    match result {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Secret(e.to_string())),
    }
}

/// Map a Keychain delete: removing a key that isn't there is a no-op success,
/// so the settings UI's "remove key" action is idempotent.
fn map_delete(result: keyring::Result<()>) -> Result<()> {
    match result {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Secret(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A representative non-NoEntry backend error for the "real failure" arm.
    fn backend_error() -> keyring::Error {
        keyring::Error::Invalid("attribute".into(), "reason".into())
    }

    #[test]
    fn get_maps_present_key_to_some() {
        assert_eq!(map_get(Ok("sk-123".into())).unwrap(), Some("sk-123".into()));
    }

    #[test]
    fn get_maps_missing_entry_to_none() {
        // The common case for an unconfigured provider — must not be an error.
        assert_eq!(map_get(Err(keyring::Error::NoEntry)).unwrap(), None);
    }

    #[test]
    fn get_surfaces_other_errors_as_secret_error() {
        let err = map_get(Err(backend_error())).unwrap_err();
        assert!(matches!(err, AppError::Secret(_)));
    }

    #[test]
    fn delete_is_idempotent() {
        assert!(map_delete(Ok(())).is_ok());
        assert!(map_delete(Err(keyring::Error::NoEntry)).is_ok());
    }

    #[test]
    fn delete_surfaces_other_errors_as_secret_error() {
        let err = map_delete(Err(backend_error())).unwrap_err();
        assert!(matches!(err, AppError::Secret(_)));
    }

    #[test]
    fn set_error_message_excludes_the_key_value() {
        // Even if the Keychain rejects the write, the surfaced error is derived
        // solely from the backend error — the key we passed is never in scope
        // for the message (FR-051/FR-052).
        let backend = backend_error();
        let expected = backend.to_string();
        let AppError::Secret(msg) = map_set(Err(backend)).unwrap_err() else {
            panic!("expected AppError::Secret");
        };
        assert_eq!(msg, expected);
    }

    #[test]
    fn error_message_comes_from_the_backend_not_the_key() {
        // FR-051/FR-052: the mapped error text is the Keychain error's own
        // Display, so a key value can never end up in a logged/returned error.
        let backend = backend_error();
        let expected = backend.to_string();
        let AppError::Secret(msg) = map_get(Err(backend)).unwrap_err() else {
            panic!("expected AppError::Secret");
        };
        assert_eq!(msg, expected);
    }
}
