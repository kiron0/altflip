pub mod error;
pub mod fsutil;
pub mod jwt;
pub mod model;
pub mod process;
pub mod resource;
pub mod secrets;
pub mod statedb;
pub mod vault;

pub use error::{Error, Result};
pub use model::*;

pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn new_id() -> String {
    ulid::Ulid::new().to_string().to_lowercase()
}

pub fn short_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))[..12].to_string()
}

pub fn home_dir() -> Result<std::path::PathBuf> {
    dirs::home_dir().ok_or_else(|| Error::Platform("Could not determine the home directory".into()))
}
