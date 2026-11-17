use ts_rs::TS;
use crate::{Error, Result};
use std::collections::HashMap;
use std::sync::Mutex;

pub trait SecretStore: Send + Sync {
    fn get(&self, service: &str, account: &str) -> Result<Option<Vec<u8>>>;
    fn set(&self, service: &str, account: &str, value: &[u8]) -> Result<()>;
    fn delete(&self, service: &str, account: &str) -> Result<()>;
    fn status(&self) -> SecretStoreStatus;
}

#[derive(Debug, Clone, serde::Serialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct SecretStoreStatus {
    pub backend: String,
    pub available: bool,
    pub message: Option<String>,
}

#[derive(Default)]
pub struct MemorySecrets {
    items: Mutex<HashMap<(String, String), Vec<u8>>>,
}

impl SecretStore for MemorySecrets {
    fn get(&self, service: &str, account: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.items.lock().unwrap().get(&(service.into(), account.into())).cloned())
    }
    fn set(&self, service: &str, account: &str, value: &[u8]) -> Result<()> {
        self.items.lock().unwrap().insert((service.into(), account.into()), value.to_vec());
        Ok(())
    }
    fn delete(&self, service: &str, account: &str) -> Result<()> {
        self.items.lock().unwrap().remove(&(service.into(), account.into()));
        Ok(())
    }
    fn status(&self) -> SecretStoreStatus {
        SecretStoreStatus { backend: "memory".into(), available: true, message: None }
    }
}

pub struct SystemSecrets;

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};

    const SECURITY: &str = "/usr/bin/security";

    fn quote(s: &str) -> Result<String> {
        if s.contains('"') || s.contains('\\') || s.contains('\n') {
            return Err(Error::Invalid("Unsupported characters in credential name".into()));
        }
        Ok(format!("\"{s}\""))
    }

    fn stored_as_binary(service: &str, account: &str) -> Result<bool> {
        let out = Command::new(SECURITY)
            .args(["find-generic-password", "-s", service, "-a", account, "-g"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .output()
            .map_err(|e| Error::Credential(format!("Cannot run the macOS security tool: {e}")))?;
        Ok(String::from_utf8_lossy(&out.stderr).lines().any(|l| l.starts_with("password: 0x")))
    }

    pub fn get(service: &str, account: &str) -> Result<Option<Vec<u8>>> {
        let out = Command::new(SECURITY)
            .args(["find-generic-password", "-s", service, "-a", account, "-w"])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| Error::Credential(format!("Cannot run the macOS security tool: {e}")))?;
        if out.status.code() == Some(44) {
            return Ok(None);
        }
        if !out.status.success() {
            let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(Error::Credential(format!("Keychain read failed for {service}: {msg}")));
        }
        let mut v = out.stdout;
        if v.last() == Some(&b'\n') {
            v.pop();
        }
        let maybe_hex = !v.is_empty() && v.len() % 2 == 0 && v.iter().all(u8::is_ascii_hexdigit);
        if maybe_hex && stored_as_binary(service, account)? {
            let raw = hex::decode(&v)
                .map_err(|_| Error::Credential(format!("Keychain item {service} could not be decoded")))?;
            return Ok(Some(raw));
        }
        Ok(Some(v))
    }

    const STDIN_LINE_LIMIT: usize = 3800;

    fn write_raw(service: &str, account: &str, value: &[u8]) -> Result<()> {
        let hexed = hex::encode(value);
        let line = format!("add-generic-password -U -s {} -a {} -X {}\n", quote(service)?, quote(account)?, hexed);
        let out = if line.len() < STDIN_LINE_LIMIT {
            let mut child = Command::new(SECURITY)
                .arg("-i")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| Error::Credential(format!("Cannot run the macOS security tool: {e}")))?;
            {
                let mut stdin =
                    child.stdin.take().ok_or_else(|| Error::Credential("security stdin unavailable".into()))?;
                stdin
                    .write_all(line.as_bytes())
                    .map_err(|e| Error::Credential(format!("Keychain write failed: {e}")))?;
            }
            child.wait_with_output().map_err(|e| Error::Credential(format!("Keychain write failed: {e}")))?
        } else {
            Command::new(SECURITY)
                .args(["add-generic-password", "-U", "-s", service, "-a", account, "-X", &hexed])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .output()
                .map_err(|e| Error::Credential(format!("Cannot run the macOS security tool: {e}")))?
        };
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if !out.status.success() || !stderr.is_empty() {
            return Err(Error::Credential(format!("Keychain write failed for {service}: {stderr}")));
        }
        Ok(())
    }

    pub fn set(service: &str, account: &str, value: &[u8]) -> Result<()> {
        let previous = get(service, account)?;
        let written = write_raw(service, account, value);
        let ok = written.is_ok() && matches!(get(service, account), Ok(Some(ref v)) if v == value);
        if ok {
            return Ok(());
        }
        if let Some(prev) = previous {
            let _ = write_raw(service, account, &prev);
        }
        written?;
        Err(Error::Verification(format!(
            "Keychain item {service} did not read back as written; the previous value was restored"
        )))
    }

    pub fn delete(service: &str, account: &str) -> Result<()> {
        let out = Command::new(SECURITY)
            .args(["delete-generic-password", "-s", service, "-a", account])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| Error::Credential(format!("Cannot run the macOS security tool: {e}")))?;
        if out.status.success() || out.status.code() == Some(44) {
            Ok(())
        } else {
            Err(Error::Credential(format!(
                "Keychain delete failed for {service}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )))
        }
    }

    pub fn status() -> SecretStoreStatus {
        let ok = std::path::Path::new(SECURITY).exists();
        SecretStoreStatus {
            backend: "macOS Keychain".into(),
            available: ok,
            message: (!ok).then(|| "The macOS security tool is missing".into()),
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::*;

    fn entry(service: &str, account: &str) -> Result<keyring::Entry> {
        #[cfg(target_os = "windows")]
        let e = keyring::Entry::new_with_target(&format!("{service}:{account}"), service, account);
        #[cfg(not(target_os = "windows"))]
        let e = keyring::Entry::new(service, account);
        e.map_err(|e| Error::Credential(format!("Credential store unavailable: {e}")))
    }

    pub fn get(service: &str, account: &str) -> Result<Option<Vec<u8>>> {
        match entry(service, account)?.get_secret() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(Error::Credential(format!("Credential read failed for {service}: {e}"))),
        }
    }

    pub fn set(service: &str, account: &str, value: &[u8]) -> Result<()> {
        let e = entry(service, account)?;
        e.set_secret(value).map_err(|e| Error::Credential(format!("Credential write failed for {service}: {e}")))?;
        match e.get_secret() {
            Ok(v) if v == value => Ok(()),
            _ => Err(Error::Verification(format!("Credential {service} did not read back as written"))),
        }
    }

    pub fn delete(service: &str, account: &str) -> Result<()> {
        match entry(service, account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(Error::Credential(format!("Credential delete failed for {service}: {e}"))),
        }
    }

    pub fn status() -> SecretStoreStatus {
        let backend = if cfg!(windows) { "Windows Credential Manager" } else { "Secret Service" };
        match entry("io.coreify.altflip.probe", "status").and_then(|e| match e.get_secret() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(Error::Credential(err.to_string())),
        }) {
            Ok(()) => SecretStoreStatus { backend: backend.into(), available: true, message: None },
            Err(e) => SecretStoreStatus { backend: backend.into(), available: false, message: Some(e.to_string()) },
        }
    }
}

impl SecretStore for SystemSecrets {
    fn get(&self, service: &str, account: &str) -> Result<Option<Vec<u8>>> {
        imp::get(service, account)
    }
    fn set(&self, service: &str, account: &str, value: &[u8]) -> Result<()> {
        imp::set(service, account, value)
    }
    fn delete(&self, service: &str, account: &str) -> Result<()> {
        imp::delete(service, account)
    }
    fn status(&self) -> SecretStoreStatus {
        imp::status()
    }
}
