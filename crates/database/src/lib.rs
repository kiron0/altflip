use altflip_core::{now_rfc3339, AuthHealth, Error, ProviderId, Result, UsageReading};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::Path;
use ts_rs::TS;

const MIGRATIONS: &[&str] = &[
    r#"
CREATE TABLE providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    installed_version TEXT,
    integration_version TEXT NOT NULL,
    compatibility_status TEXT NOT NULL,
    last_detected_at TEXT
);
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES providers(id),
    label TEXT NOT NULL,
    category TEXT,
    email TEXT,
    account_identifier TEXT NOT NULL,
    display_name TEXT,
    organization TEXT,
    subscription_type TEXT,
    auth_method TEXT,
    profile_reference TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'ready',
    health TEXT NOT NULL DEFAULT 'unknown',
    favorite INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    last_authenticated_at TEXT,
    last_used_at TEXT,
    last_captured_at TEXT,
    last_error TEXT,
    UNIQUE (provider_id, account_identifier)
);
CREATE TABLE active_accounts (
    provider_id TEXT PRIMARY KEY REFERENCES providers(id),
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    activation_state TEXT NOT NULL,
    verified_at TEXT
);
CREATE TABLE usage_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id TEXT NOT NULL REFERENCES providers(id),
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    usage_type TEXT NOT NULL,
    label TEXT NOT NULL,
    used_percent REAL,
    used_value REAL,
    limit_value REAL,
    window_minutes INTEGER,
    reset_at TEXT,
    data_source TEXT NOT NULL,
    confidence TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    retrieved_at TEXT NOT NULL,
    UNIQUE (account_id, usage_type, observed_at)
);
CREATE TABLE switch_history (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES providers(id),
    from_account_id TEXT,
    to_account_id TEXT,
    from_label TEXT,
    to_label TEXT,
    status TEXT NOT NULL,
    switch_mode TEXT NOT NULL,
    error_code TEXT,
    message TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT
);
CREATE TABLE recovery_snapshots (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES providers(id),
    operation_id TEXT NOT NULL REFERENCES switch_history(id),
    snapshot_reference TEXT NOT NULL,
    created_at TEXT NOT NULL,
    status TEXT NOT NULL
);
CREATE TABLE activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    at TEXT NOT NULL,
    provider_id TEXT,
    account_id TEXT,
    kind TEXT NOT NULL,
    message TEXT NOT NULL
);
CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_accounts_provider ON accounts(provider_id, sort_order);
CREATE INDEX idx_history_started ON switch_history(started_at DESC);
CREATE INDEX idx_activity_at ON activity(at DESC);
CREATE INDEX idx_usage_account ON usage_snapshots(account_id, observed_at DESC);
"#,
    r#"
DELETE FROM usage_snapshots;
"#,
    r#"
DELETE FROM recovery_snapshots WHERE provider_id = 'opencode';
DELETE FROM switch_history WHERE provider_id = 'opencode';
DELETE FROM active_accounts WHERE provider_id = 'opencode';
DELETE FROM usage_snapshots WHERE provider_id = 'opencode';
DELETE FROM accounts WHERE provider_id = 'opencode';
DELETE FROM activity WHERE provider_id = 'opencode';
DELETE FROM providers WHERE id = 'opencode';
"#,
];

fn present<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}

fn db_err(e: rusqlite::Error) -> Error {
    Error::Database(e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct AccountRow {
    pub id: String,
    pub provider: ProviderId,
    pub label: String,
    pub category: Option<String>,
    pub email: Option<String>,
    pub account_identifier: String,
    pub display_name: Option<String>,
    pub organization: Option<String>,
    pub subscription_type: Option<String>,
    pub auth_method: Option<String>,
    pub profile_reference: String,
    pub status: String,
    pub health: AuthHealth,
    pub favorite: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub last_authenticated_at: Option<String>,
    pub last_used_at: Option<String>,
    pub last_captured_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveRow {
    pub provider: ProviderId,
    pub account_id: String,
    pub activation_state: String,
    pub verified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct SwitchRecord {
    pub id: String,
    pub provider: ProviderId,
    pub from_account_id: Option<String>,
    pub to_account_id: Option<String>,
    pub from_label: Option<String>,
    pub to_label: Option<String>,
    pub status: String,
    pub switch_mode: String,
    pub error_code: Option<String>,
    pub message: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryRow {
    pub id: String,
    pub provider: ProviderId,
    pub operation_id: String,
    pub snapshot_reference: String,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct ActivityRow {
    pub id: i64,
    pub at: String,
    pub provider: Option<ProviderId>,
    pub account_id: Option<String>,
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct UsageRow {
    pub account_id: String,
    pub provider: ProviderId,
    #[serde(flatten)]
    pub reading: UsageReading,
    pub retrieved_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export_to = "bindings.ts", optional_fields)]
#[serde(rename_all = "camelCase")]
pub struct AccountPatch {
    pub label: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub category: Option<Option<String>>,
    pub favorite: Option<bool>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct NewAccount {
    pub provider: ProviderId,
    pub label: String,
    pub category: Option<String>,
    pub identity: altflip_core::Identity,
    pub profile_reference: String,
    pub health: AuthHealth,
}

fn provider_col(row: &Row, idx: &str) -> rusqlite::Result<ProviderId> {
    let s: String = row.get(idx)?;
    ProviderId::parse(&s).ok_or_else(|| rusqlite::Error::InvalidColumnType(0, idx.into(), rusqlite::types::Type::Text))
}

fn account_from(row: &Row) -> rusqlite::Result<AccountRow> {
    Ok(AccountRow {
        id: row.get("id")?,
        provider: provider_col(row, "provider_id")?,
        label: row.get("label")?,
        category: row.get("category")?,
        email: row.get("email")?,
        account_identifier: row.get("account_identifier")?,
        display_name: row.get("display_name")?,
        organization: row.get("organization")?,
        subscription_type: row.get("subscription_type")?,
        auth_method: row.get("auth_method")?,
        profile_reference: row.get("profile_reference")?,
        status: row.get("status")?,
        health: AuthHealth::parse(&row.get::<_, String>("health")?),
        favorite: row.get::<_, i64>("favorite")? != 0,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
        last_authenticated_at: row.get("last_authenticated_at")?,
        last_used_at: row.get("last_used_at")?,
        last_captured_at: row.get("last_captured_at")?,
        last_error: row.get("last_error")?,
    })
}

fn switch_from(row: &Row) -> rusqlite::Result<SwitchRecord> {
    Ok(SwitchRecord {
        id: row.get("id")?,
        provider: provider_col(row, "provider_id")?,
        from_account_id: row.get("from_account_id")?,
        to_account_id: row.get("to_account_id")?,
        from_label: row.get("from_label")?,
        to_label: row.get("to_label")?,
        status: row.get("status")?,
        switch_mode: row.get("switch_mode")?,
        error_code: row.get("error_code")?,
        message: row.get("message")?,
        started_at: row.get("started_at")?,
        completed_at: row.get("completed_at")?,
    })
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Error::io(format!("Cannot create {}", dir.display()), e))?;
        }
        let conn = Connection::open(path).map_err(db_err)?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory().map_err(db_err)?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL;")
            .map_err(db_err)?;
        conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(db_err)?;
        let mut store = Store { conn };
        store.migrate()?;
        for p in ProviderId::ALL {
            store
                .conn
                .execute(
                    "INSERT OR IGNORE INTO providers (id, name, integration_version, compatibility_status) VALUES (?1, ?2, ?3, 'unknown')",
                    params![p.as_str(), p.name(), env!("CARGO_PKG_VERSION")],
                )
                .map_err(db_err)?;
        }
        Ok(store)
    }

    fn migrate(&mut self) -> Result<()> {
        let version: i64 = self.conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(db_err)?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = self.conn.transaction().map_err(db_err)?;
            tx.execute_batch(sql).map_err(db_err)?;
            tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1)).map_err(db_err)?;
            tx.commit().map_err(db_err)?;
        }
        Ok(())
    }

    pub fn integrity_check(&self) -> Result<String> {
        self.conn.query_row("PRAGMA integrity_check", [], |r| r.get(0)).map_err(db_err)
    }

    pub fn update_provider(&self, p: ProviderId, version: Option<&str>, compat: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE providers SET installed_version = ?2, compatibility_status = ?3, last_detected_at = ?4, integration_version = ?5
                 WHERE id = ?1 AND (installed_version IS NOT ?2 OR compatibility_status IS NOT ?3 OR integration_version IS NOT ?5 OR last_detected_at IS NULL)",
                params![p.as_str(), version, compat, now_rfc3339(), env!("CARGO_PKG_VERSION")],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn accounts(&self, provider: Option<ProviderId>) -> Result<Vec<AccountRow>> {
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT * FROM accounts WHERE (?1 IS NULL OR provider_id = ?1) ORDER BY provider_id, favorite DESC, sort_order, created_at",
            )
            .map_err(db_err)?;
        let rows = stmt.query_map(params![provider.map(|p| p.as_str())], account_from).map_err(db_err)?;
        rows.collect::<rusqlite::Result<_>>().map_err(db_err)
    }

    pub fn account(&self, id: &str) -> Result<AccountRow> {
        self.conn
            .query_row("SELECT * FROM accounts WHERE id = ?1", params![id], account_from)
            .optional()
            .map_err(db_err)?
            .ok_or_else(|| Error::NotFound("That account no longer exists".into()))
    }

    pub fn account_by_identity(&self, provider: ProviderId, key: &str) -> Result<Option<AccountRow>> {
        self.conn
            .query_row(
                "SELECT * FROM accounts WHERE provider_id = ?1 AND account_identifier = ?2",
                params![provider.as_str(), key],
                account_from,
            )
            .optional()
            .map_err(db_err)
    }

    pub fn insert_account(&self, id: &str, n: &NewAccount) -> Result<AccountRow> {
        let now = now_rfc3339();
        let order: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM accounts WHERE provider_id = ?1",
                params![n.provider.as_str()],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        self.conn
            .execute(
                "INSERT INTO accounts (id, provider_id, label, category, email, account_identifier, display_name, organization,
                 subscription_type, auth_method, profile_reference, health, sort_order, created_at, last_authenticated_at, last_captured_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14, ?14)",
                params![
                    id,
                    n.provider.as_str(),
                    n.label,
                    n.category,
                    n.identity.email,
                    n.identity.key,
                    n.identity.display_name,
                    n.identity.organization,
                    n.identity.plan,
                    n.identity.auth_method,
                    n.profile_reference,
                    n.health.as_str(),
                    order,
                    now
                ],
            )
            .map_err(db_err)?;
        self.account(id)
    }

    pub fn refresh_identity(&self, id: &str, identity: &altflip_core::Identity, health: AuthHealth) -> Result<()> {
        self.conn
            .execute(
                "UPDATE accounts SET email = COALESCE(?2, email), display_name = COALESCE(?3, display_name),
                 organization = COALESCE(?4, organization), subscription_type = COALESCE(?5, subscription_type),
                 auth_method = COALESCE(?6, auth_method), health = ?7, last_captured_at = ?8 WHERE id = ?1",
                params![
                    id,
                    identity.email,
                    identity.display_name,
                    identity.organization,
                    identity.plan,
                    identity.auth_method,
                    health.as_str(),
                    now_rfc3339()
                ],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn set_plan(&self, id: &str, plan: &str) -> Result<()> {
        self.conn
            .execute("UPDATE accounts SET subscription_type = ?2 WHERE id = ?1", params![id, plan])
            .map_err(db_err)?;
        Ok(())
    }

    pub fn set_health(&self, id: &str, health: AuthHealth) -> Result<()> {
        self.conn
            .execute("UPDATE accounts SET health = ?2 WHERE id = ?1", params![id, health.as_str()])
            .map_err(db_err)?;
        Ok(())
    }

    pub fn set_account_error(&self, id: &str, err: Option<&str>) -> Result<()> {
        self.conn.execute("UPDATE accounts SET last_error = ?2 WHERE id = ?1", params![id, err]).map_err(db_err)?;
        Ok(())
    }

    pub fn mark_used(&self, id: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE accounts SET last_used_at = ?2, last_error = NULL WHERE id = ?1",
                params![id, now_rfc3339()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn patch_account(&self, id: &str, p: &AccountPatch) -> Result<AccountRow> {
        let current = self.account(id)?;
        let label = p.label.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or(&current.label);
        let category = p.category.clone().unwrap_or(current.category);
        self.conn
            .execute(
                "UPDATE accounts SET label = ?2, category = ?3, favorite = ?4, sort_order = ?5 WHERE id = ?1",
                params![
                    id,
                    label,
                    category,
                    p.favorite.unwrap_or(current.favorite) as i64,
                    p.sort_order.unwrap_or(current.sort_order)
                ],
            )
            .map_err(db_err)?;
        self.account(id)
    }

    pub fn delete_account(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM accounts WHERE id = ?1", params![id]).map_err(db_err)?;
        Ok(())
    }

    pub fn active(&self) -> Result<Vec<ActiveRow>> {
        let mut stmt = self.conn.prepare_cached("SELECT * FROM active_accounts").map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ActiveRow {
                    provider: provider_col(r, "provider_id")?,
                    account_id: r.get("account_id")?,
                    activation_state: r.get("activation_state")?,
                    verified_at: r.get("verified_at")?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<rusqlite::Result<_>>().map_err(db_err)
    }

    pub fn active_for(&self, p: ProviderId) -> Result<Option<ActiveRow>> {
        Ok(self.active()?.into_iter().find(|a| a.provider == p))
    }

    pub fn set_active(&self, p: ProviderId, account_id: &str, state: &str, verified: bool) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO active_accounts (provider_id, account_id, activation_state, verified_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(provider_id) DO UPDATE SET account_id = excluded.account_id, activation_state = excluded.activation_state,
                 verified_at = COALESCE(excluded.verified_at, active_accounts.verified_at)",
                params![p.as_str(), account_id, state, verified.then(now_rfc3339)],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn set_activation_state(&self, p: ProviderId, state: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE active_accounts SET activation_state = ?2 WHERE provider_id = ?1",
                params![p.as_str(), state],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn clear_active(&self, p: ProviderId) -> Result<()> {
        self.conn.execute("DELETE FROM active_accounts WHERE provider_id = ?1", params![p.as_str()]).map_err(db_err)?;
        Ok(())
    }

    pub fn begin_switch(&self, r: &SwitchRecord) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO switch_history (id, provider_id, from_account_id, to_account_id, from_label, to_label, status, switch_mode, started_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    r.id,
                    r.provider.as_str(),
                    r.from_account_id,
                    r.to_account_id,
                    r.from_label,
                    r.to_label,
                    r.status,
                    r.switch_mode,
                    r.started_at
                ],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn set_switch_status(&self, id: &str, status: &str) -> Result<()> {
        self.conn
            .execute("UPDATE switch_history SET status = ?2 WHERE id = ?1", params![id, status])
            .map_err(db_err)?;
        Ok(())
    }

    pub fn finish_switch(&self, id: &str, status: &str, code: Option<&str>, message: Option<&str>) -> Result<()> {
        self.conn
            .execute(
                "UPDATE switch_history SET status = ?2, error_code = ?3, message = ?4, completed_at = ?5 WHERE id = ?1",
                params![id, status, code, message, now_rfc3339()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn switch_history(&self, limit: i64) -> Result<Vec<SwitchRecord>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT * FROM switch_history ORDER BY started_at DESC LIMIT ?1")
            .map_err(db_err)?;
        let rows = stmt.query_map(params![limit], switch_from).map_err(db_err)?;
        rows.collect::<rusqlite::Result<_>>().map_err(db_err)
    }

    pub fn switch_record(&self, id: &str) -> Result<SwitchRecord> {
        self.conn
            .query_row("SELECT * FROM switch_history WHERE id = ?1", params![id], switch_from)
            .optional()
            .map_err(db_err)?
            .ok_or_else(|| Error::NotFound("That switch record no longer exists".into()))
    }

    pub fn last_switch_at(&self, p: ProviderId) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT MAX(completed_at) FROM switch_history WHERE provider_id = ?1 AND status IN ('succeeded', 'recovered')",
                params![p.as_str()],
                |r| r.get(0),
            )
            .map_err(db_err)
    }

    pub fn add_recovery(&self, r: &RecoveryRow) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO recovery_snapshots (id, provider_id, operation_id, snapshot_reference, created_at, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![r.id, r.provider.as_str(), r.operation_id, r.snapshot_reference, r.created_at, r.status],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn set_recovery_status(&self, operation_id: &str, status: &str) -> Result<()> {
        self.conn
            .execute("UPDATE recovery_snapshots SET status = ?2 WHERE operation_id = ?1", params![operation_id, status])
            .map_err(db_err)?;
        Ok(())
    }

    pub fn recoveries(&self, status: Option<&str>) -> Result<Vec<RecoveryRow>> {
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT * FROM recovery_snapshots WHERE (?1 IS NULL OR status = ?1) ORDER BY created_at DESC",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![status], |r| {
                Ok(RecoveryRow {
                    id: r.get("id")?,
                    provider: provider_col(r, "provider_id")?,
                    operation_id: r.get("operation_id")?,
                    snapshot_reference: r.get("snapshot_reference")?,
                    created_at: r.get("created_at")?,
                    status: r.get("status")?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<rusqlite::Result<_>>().map_err(db_err)
    }

    pub fn delete_recovery(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM recovery_snapshots WHERE id = ?1", params![id]).map_err(db_err)?;
        Ok(())
    }

    pub fn incomplete_switches(&self) -> Result<Vec<SwitchRecord>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT * FROM switch_history WHERE status IN ('started', 'applying') ORDER BY started_at")
            .map_err(db_err)?;
        let rows = stmt.query_map([], switch_from).map_err(db_err)?;
        rows.collect::<rusqlite::Result<_>>().map_err(db_err)
    }

    pub fn log(&self, provider: Option<ProviderId>, account_id: Option<&str>, kind: &str, message: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO activity (at, provider_id, account_id, kind, message) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![now_rfc3339(), provider.map(|p| p.as_str()), account_id, kind, message],
            )
            .map_err(db_err)?;
        let id = self.conn.last_insert_rowid();
        if id % 100 == 0 {
            self.conn
                .prepare_cached("DELETE FROM activity WHERE id <= ?1")
                .and_then(|mut s| s.execute(params![id - 2000]))
                .map_err(db_err)?;
        }
        Ok(())
    }

    pub fn reorder(&self, provider: ProviderId, ids: &[String]) -> Result<()> {
        let tx = self.conn.unchecked_transaction().map_err(db_err)?;
        {
            let mut stmt = tx
                .prepare_cached("UPDATE accounts SET sort_order = ?3 WHERE id = ?1 AND provider_id = ?2")
                .map_err(db_err)?;
            for (i, id) in ids.iter().enumerate() {
                if stmt.execute(params![id, provider.as_str(), i as i64]).map_err(db_err)? != 1 {
                    return Err(Error::Invalid("Accounts belong to different providers".into()));
                }
            }
        }
        tx.commit().map_err(db_err)
    }

    pub fn activity(&self, limit: i64) -> Result<Vec<ActivityRow>> {
        let mut stmt = self.conn.prepare_cached("SELECT * FROM activity ORDER BY id DESC LIMIT ?1").map_err(db_err)?;
        let rows = stmt
            .query_map(params![limit], |r| {
                let p: Option<String> = r.get("provider_id")?;
                Ok(ActivityRow {
                    id: r.get("id")?,
                    at: r.get("at")?,
                    provider: p.as_deref().and_then(ProviderId::parse),
                    account_id: r.get("account_id")?,
                    kind: r.get("kind")?,
                    message: r.get("message")?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<rusqlite::Result<_>>().map_err(db_err)
    }

    pub fn add_usage_many(&self, provider: ProviderId, rows: &[(String, &UsageReading)]) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let tx = self.conn.unchecked_transaction().map_err(db_err)?;
        for (account_id, u) in rows {
            self.add_usage(provider, account_id, u)?;
        }
        tx.commit().map_err(db_err)
    }

    pub fn add_usage(&self, provider: ProviderId, account_id: &str, u: &UsageReading) -> Result<()> {
        self.conn
            .prepare_cached(
                "INSERT OR IGNORE INTO usage_snapshots (provider_id, account_id, usage_type, label, used_percent, used_value, limit_value,
                 window_minutes, reset_at, data_source, confidence, observed_at, retrieved_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )
            .and_then(|mut s| {
                s.execute(params![
                    provider.as_str(),
                    account_id,
                    u.usage_type,
                    u.label,
                    u.used_percent,
                    u.used_value,
                    u.limit_value,
                    u.window_minutes,
                    u.reset_at,
                    u.source,
                    u.confidence,
                    u.observed_at,
                    now_rfc3339()
                ])
            })
            .map_err(db_err)?;
        Ok(())
    }

    pub fn drop_usage_source(&self, provider: ProviderId, account_id: Option<&str>, source: &str) -> Result<usize> {
        self.conn
            .execute(
                "DELETE FROM usage_snapshots WHERE provider_id = ?1 AND data_source = ?2 AND (?3 IS NULL OR account_id = ?3)",
                params![provider.as_str(), source, account_id],
            )
            .map_err(db_err)
    }

    pub fn prune_usage(&self, keep_days: i64) -> Result<usize> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(keep_days))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        self.conn
            .execute(
                "DELETE FROM usage_snapshots WHERE observed_at < ?1 AND id NOT IN (
                    SELECT u.id FROM usage_snapshots u JOIN (
                        SELECT account_id, usage_type, MAX(observed_at) AS m FROM usage_snapshots GROUP BY account_id, usage_type
                    ) l ON l.account_id = u.account_id AND l.usage_type = u.usage_type AND l.m = u.observed_at
                 )",
                params![cutoff],
            )
            .map_err(db_err)
    }

    pub fn latest_usage(&self) -> Result<Vec<UsageRow>> {
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT u.* FROM usage_snapshots u JOIN (
                    SELECT account_id, usage_type, MAX(observed_at) AS m FROM usage_snapshots GROUP BY account_id, usage_type
                 ) l ON l.account_id = u.account_id AND l.usage_type = u.usage_type AND l.m = u.observed_at",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(UsageRow {
                    account_id: r.get("account_id")?,
                    provider: provider_col(r, "provider_id")?,
                    retrieved_at: r.get("retrieved_at")?,
                    reading: UsageReading {
                        usage_type: r.get("usage_type")?,
                        label: r.get("label")?,
                        used_percent: r.get("used_percent")?,
                        used_value: r.get("used_value")?,
                        limit_value: r.get("limit_value")?,
                        window_minutes: r.get("window_minutes")?,
                        reset_at: r.get("reset_at")?,
                        source: r.get("data_source")?,
                        confidence: r.get("confidence")?,
                        observed_at: r.get("observed_at")?,
                        account_key: None,
                    },
                })
            })
            .map_err(db_err)?;
        rows.collect::<rusqlite::Result<_>>().map_err(db_err)
    }

    pub fn settings(&self) -> Result<serde_json::Map<String, serde_json::Value>> {
        let mut stmt = self.conn.prepare_cached("SELECT key, value FROM app_settings").map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(db_err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_err)?;
        Ok(rows.into_iter().filter_map(|(k, v)| serde_json::from_str(&v).ok().map(|v| (k, v))).collect())
    }

    pub fn setting(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let v: Option<String> = self
            .conn
            .query_row("SELECT value FROM app_settings WHERE key = ?1", params![key], |r| r.get(0))
            .optional()
            .map_err(db_err)?;
        Ok(v.and_then(|s| serde_json::from_str(&s).ok()))
    }

    pub fn set_setting(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                params![key, value.to_string(), now_rfc3339()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn reset_settings(&self) -> Result<()> {
        self.conn.execute("DELETE FROM app_settings", []).map_err(db_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altflip_core::Identity;

    fn new(p: ProviderId, key: &str) -> NewAccount {
        NewAccount {
            provider: p,
            label: format!("L {key}"),
            category: None,
            identity: Identity { key: key.into(), email: Some(format!("{key}@x.dev")), ..Default::default() },
            profile_reference: format!("accounts/{key}"),
            health: AuthHealth::Ok,
        }
    }

    #[test]
    fn accounts_unique_per_provider_identity() {
        let s = Store::open_in_memory().unwrap();
        s.insert_account("a", &new(ProviderId::Claude, "k1")).unwrap();
        assert!(s.insert_account("b", &new(ProviderId::Claude, "k1")).is_err());
        s.insert_account("c", &new(ProviderId::Codex, "k1")).unwrap();
        assert_eq!(s.accounts(None).unwrap().len(), 2);
        assert_eq!(s.account_by_identity(ProviderId::Codex, "k1").unwrap().unwrap().id, "c");
    }

    #[test]
    fn one_active_per_provider_and_cascade() {
        let s = Store::open_in_memory().unwrap();
        s.insert_account("a", &new(ProviderId::Claude, "k1")).unwrap();
        s.insert_account("b", &new(ProviderId::Claude, "k2")).unwrap();
        s.set_active(ProviderId::Claude, "a", "verified", true).unwrap();
        s.set_active(ProviderId::Claude, "b", "verified", true).unwrap();
        assert_eq!(s.active().unwrap().len(), 1);
        assert_eq!(s.active_for(ProviderId::Claude).unwrap().unwrap().account_id, "b");
        s.delete_account("b").unwrap();
        assert!(s.active_for(ProviderId::Claude).unwrap().is_none());
    }

    #[test]
    fn prune_keeps_latest_reading() {
        let s = Store::open_in_memory().unwrap();
        s.insert_account("a", &new(ProviderId::Claude, "k1")).unwrap();
        let reading = |at: &str, pct: f64| UsageReading {
            usage_type: "five_hour".into(),
            label: "5-hour usage".into(),
            used_percent: Some(pct),
            used_value: None,
            limit_value: None,
            window_minutes: Some(300),
            reset_at: None,
            source: "Anthropic".into(),
            confidence: "provider-reported".into(),
            observed_at: at.into(),
            account_key: None,
        };
        s.add_usage(ProviderId::Claude, "a", &reading("2020-01-01T00:00:00.000Z", 10.0)).unwrap();
        s.add_usage(ProviderId::Claude, "a", &reading("2020-01-02T00:00:00.000Z", 20.0)).unwrap();
        assert_eq!(s.prune_usage(7).unwrap(), 1);
        let rows = s.latest_usage().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].reading.used_percent, Some(20.0));
    }

    #[test]
    fn patch_null_category_clears_it() {
        let p: AccountPatch = serde_json::from_str(r#"{"category":null}"#).unwrap();
        assert_eq!(p.category, Some(None));
        let p: AccountPatch = serde_json::from_str(r#"{"label":"x"}"#).unwrap();
        assert_eq!(p.category, None);
    }

    #[test]
    fn reorder_is_atomic_and_provider_scoped() {
        let s = Store::open_in_memory().unwrap();
        s.insert_account("a", &new(ProviderId::Claude, "k1")).unwrap();
        s.insert_account("b", &new(ProviderId::Claude, "k2")).unwrap();
        s.insert_account("c", &new(ProviderId::Codex, "k3")).unwrap();
        s.reorder(ProviderId::Claude, &["b".into(), "a".into()]).unwrap();
        assert_eq!(s.account("b").unwrap().sort_order, 0);
        assert_eq!(s.account("a").unwrap().sort_order, 1);
        assert!(s.reorder(ProviderId::Claude, &["a".into(), "c".into()]).is_err());
        assert_eq!(s.account("a").unwrap().sort_order, 1);
        for i in 0..2105 {
            s.log(None, None, "t", &format!("m{i}")).unwrap();
        }
        let n = s.activity(5000).unwrap().len();
        assert!((2000..=2100).contains(&n));
    }

    #[test]
    fn rejects_dangling_active() {
        let s = Store::open_in_memory().unwrap();
        assert!(s.set_active(ProviderId::Codex, "nope", "verified", true).is_err());
    }

    #[test]
    fn switch_journal_and_settings() {
        let s = Store::open_in_memory().unwrap();
        let r = SwitchRecord {
            id: "op1".into(),
            provider: ProviderId::Codex,
            from_account_id: None,
            to_account_id: None,
            from_label: None,
            to_label: Some("Work".into()),
            status: "started".into(),
            switch_mode: "new-sessions".into(),
            error_code: None,
            message: None,
            started_at: now_rfc3339(),
            completed_at: None,
        };
        s.begin_switch(&r).unwrap();
        assert_eq!(s.incomplete_switches().unwrap().len(), 1);
        s.finish_switch("op1", "succeeded", None, None).unwrap();
        assert!(s.incomplete_switches().unwrap().is_empty());
        s.set_setting("theme", &serde_json::json!("dark")).unwrap();
        assert_eq!(s.setting("theme").unwrap().unwrap(), "dark");
        assert_eq!(s.integrity_check().unwrap(), "ok");
    }

    #[test]
    fn patch_keeps_identity() {
        let s = Store::open_in_memory().unwrap();
        s.insert_account("a", &new(ProviderId::Claude, "k1")).unwrap();
        let r = s
            .patch_account(
                "a",
                &AccountPatch { label: Some("Work".into()), favorite: Some(true), ..Default::default() },
            )
            .unwrap();
        assert_eq!(r.label, "Work");
        assert!(r.favorite);
        assert_eq!(r.account_identifier, "k1");
    }
}
