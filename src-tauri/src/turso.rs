use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "NATRA-ERP-TURSO";
const ACCOUNT_URL: &str = "database_url";
const ACCOUNT_TOKEN: &str = "auth_token";
const MAX_URL_LEN: usize = 512;
const MAX_TOKEN_LEN: usize = 16_384;

#[derive(Debug, Serialize, Deserialize)]
pub struct TursoConfig { pub database_url: String, pub configured: bool }

fn entry(account: &str) -> Result<Entry, String> { Entry::new(SERVICE, account).map_err(|e| e.to_string()) }

fn validate_url(url: &str) -> Result<String, String> {
    let value = url.trim();
    if value.is_empty() || value.len() > MAX_URL_LEN || value.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("Invalid Turso database URL".into());
    }
    if !(value.starts_with("libsql://") || value.starts_with("https://")) {
        return Err("Turso database URL must use libsql:// or https://".into());
    }
    Ok(value.to_string())
}

fn validate_token(token: &str) -> Result<String, String> {
    let value = token.trim();
    if value.is_empty() || value.len() > MAX_TOKEN_LEN || value.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("Invalid Turso auth token".into());
    }
    Ok(value.to_string())
}

pub fn save(url: &str, token: &str) -> Result<(), String> {
    let url = validate_url(url)?;
    let token = validate_token(token)?;
    entry(ACCOUNT_URL)?.set_password(&url).map_err(|e| e.to_string())?;
    if let Err(error) = entry(ACCOUNT_TOKEN)?.set_password(&token) {
        let _ = entry(ACCOUNT_URL)?.delete_credential();
        return Err(error.to_string());
    }
    Ok(())
}

pub fn status() -> Result<TursoConfig, String> {
    let url = entry(ACCOUNT_URL)?.get_password().unwrap_or_default();
    Ok(TursoConfig { configured: !url.is_empty(), database_url: url })
}

pub fn clear() -> Result<(), String> {
    let _ = entry(ACCOUNT_URL)?.delete_credential();
    let _ = entry(ACCOUNT_TOKEN)?.delete_credential();
    Ok(())
}

pub fn credentials() -> Result<(String, String), String> {
    let url = entry(ACCOUNT_URL)?.get_password().map_err(|e| e.to_string())?;
    let token = entry(ACCOUNT_TOKEN)?.get_password().map_err(|e| e.to_string())?;
    if url.is_empty() || token.is_empty() { return Err("Turso credentials are incomplete.".into()); }
    Ok((url, token))
}
