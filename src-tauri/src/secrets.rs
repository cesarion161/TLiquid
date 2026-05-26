//! Secure secret storage backed by the macOS Keychain (P0-005).
//!
//! API keys are stored per provider id and never written to the plaintext
//! settings file or logged (PRD FR-050, FR-051, FR-052).

use crate::error::{AppError, Result};

/// Keychain service name. Entries are keyed by provider id (the "account").
const SERVICE: &str = "com.tliquid.app";

fn entry(provider_id: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, provider_id).map_err(|e| AppError::Secret(e.to_string()))
}

pub fn set_key(provider_id: &str, api_key: &str) -> Result<()> {
    entry(provider_id)?
        .set_password(api_key)
        .map_err(|e| AppError::Secret(e.to_string()))
}

pub fn get_key(provider_id: &str) -> Result<Option<String>> {
    match entry(provider_id)?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Secret(e.to_string())),
    }
}

pub fn delete_key(provider_id: &str) -> Result<()> {
    match entry(provider_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Secret(e.to_string())),
    }
}
