use crate::Engine;
use altflip_core::resource::Snapshot;
use altflip_core::vault::{open_with_passphrase, seal_with_passphrase};
use altflip_core::{fsutil, AuthHealth, Error, Result};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::path::Path;
use zeroize::Zeroizing;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupAccount {
    label: String,
    category: Option<String>,
    favorite: bool,
    snapshot: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupFile {
    format: String,
    version: u32,
    exported_at: String,
    app_version: String,
    accounts: Vec<BackupAccount>,
}

pub fn export(engine: &Engine, path: &Path, passphrase: &str) -> Result<usize> {
    let rows = engine.db().accounts(None)?;
    let mut accounts = Vec::new();
    for row in &rows {
        let snap = engine.load_snapshot(row)?;
        accounts.push(BackupAccount {
            label: row.label.clone(),
            category: row.category.clone(),
            favorite: row.favorite,
            snapshot: base64::engine::general_purpose::STANDARD.encode(snap.to_bytes()?.as_slice()),
        });
    }
    let file = BackupFile {
        format: "altflip-backup".into(),
        version: 1,
        exported_at: altflip_core::now_rfc3339(),
        app_version: env!("CARGO_PKG_VERSION").into(),
        accounts,
    };
    let plain = Zeroizing::new(serde_json::to_vec(&file)?);
    let sealed = seal_with_passphrase(passphrase, &plain)?;
    fsutil::write_private(path, &sealed)?;
    engine.db().log(None, None, "backup", &format!("Exported an encrypted backup of {} accounts", rows.len()))?;
    Ok(rows.len())
}

pub fn import(engine: &Engine, path: &Path, passphrase: &str) -> Result<usize> {
    let data = std::fs::read(path).map_err(|e| Error::io("Cannot read the backup file", e))?;
    let plain = open_with_passphrase(passphrase, &data)?;
    let file: BackupFile = serde_json::from_slice(&plain)?;
    if file.format != "altflip-backup" || file.version != 1 {
        return Err(Error::Invalid("This backup was made by an unsupported version of AltFlip".into()));
    }
    let mut count = 0;
    for a in file.accounts {
        let raw = Zeroizing::new(
            base64::engine::general_purpose::STANDARD
                .decode(&a.snapshot)
                .map_err(|_| Error::Invalid("The backup contains a damaged account".into()))?,
        );
        let snap = Snapshot::from_bytes(&raw)?;
        let health = engine.provider(snap.provider).map(|p| p.health(&snap)).unwrap_or(AuthHealth::Unknown);
        let (row, _) = engine.upsert(&snap, Some(a.label), a.category, health)?;
        if a.favorite {
            engine.db().patch_account(
                &row.id,
                &altflip_database::AccountPatch { favorite: Some(true), ..Default::default() },
            )?;
        }
        count += 1;
    }
    engine.db().log(None, None, "backup", &format!("Restored {count} accounts from a backup"))?;
    Ok(count)
}
