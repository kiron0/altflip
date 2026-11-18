mod backup;
mod settings;
mod view;

pub use altflip_database::{AccountPatch, AccountRow, ActivityRow, SwitchRecord, UsageRow};
pub use settings::{default_settings, Settings};
pub use view::{Feed, FeedQuery, Notice, SwitchView, UsageView};
use ts_rs::TS;

use altflip_core::resource::{ResourceIo, Snapshot};
use altflip_core::secrets::{SecretStore, SecretStoreStatus};
use altflip_core::vault::Vault;
use altflip_core::{
    new_id, now_rfc3339, process, AuthHealth, Compatibility, Error, Identity, ProcessInfo, ProcessKind, ProviderId,
    ProviderInfo, Result, SwitchMode,
};
use altflip_database::{NewAccount, RecoveryRow, Store};
pub use altflip_providers::claude_status::StatusLineState;
use altflip_providers::{Ctx, Provider};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

const CLAUDE_ESTIMATE_SOURCE: &str = "Claude Desktop history";
const CODEX_LOG_SOURCE: &str = "Codex session log";
const CLAUDE_SIGNED_OUT: &str =
    "Claude Code needs login. Run claude auth login in Terminal to sign in first, then press Refresh.";

pub fn sign_in_hint(provider: ProviderId) -> &'static str {
    match provider {
        ProviderId::Claude => "Run claude auth login in Terminal to sign in first.",
        ProviderId::Codex => "Run codex login in Terminal to sign in first.",
        ProviderId::Antigravity => "Open Antigravity and sign in first.",
    }
}

pub struct EngineConfig {
    pub data_dir: PathBuf,
    pub home: PathBuf,
    pub secrets: Arc<dyn SecretStore>,
}

pub struct Engine {
    store: Mutex<Store>,
    vault: Vault,
    secrets: Arc<dyn SecretStore>,
    providers: Vec<Arc<dyn Provider>>,
    locks: HashMap<ProviderId, Mutex<()>>,
    logins: Mutex<HashMap<String, Arc<AtomicBool>>>,
    usage_errors: Mutex<HashMap<String, String>>,
    last_live: Mutex<HashMap<String, std::time::Instant>>,
    live_cache: Mutex<HashMap<ProviderId, LiveRead>>,
    usage_lock: Mutex<()>,
    data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    #[serde(flatten)]
    pub row: AccountRow,
    pub active: bool,
    pub activation_state: Option<String>,
    pub verified_at: Option<String>,
    pub usage: Vec<UsageView>,
    pub usage_error: Option<String>,
    pub row_state: view::RowState,
    pub state_badge: Option<altflip_core::text::Badge>,
    pub plan_label: Option<String>,
    pub subtitle: String,
    pub health_badge: altflip_core::text::Badge,
    pub needs_sign_in: bool,
    pub inline_usage: Option<view::InlineUsage>,
    pub freshness: Option<view::Freshness>,
    pub added_text: String,
    pub last_used_text: String,
    pub captured_text: String,
    pub verified_text: Option<String>,
}

type LiveRead = (Option<LiveAccount>, Option<String>);

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct LiveAccount {
    pub identity: Identity,
    pub health: AuthHealth,
    pub saved_account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct ProviderState {
    pub info: ProviderInfo,
    pub accounts: Vec<AccountView>,
    pub active_account_id: Option<String>,
    pub activation_state: Option<String>,
    pub live: Option<LiveAccount>,
    pub live_error: Option<String>,
    pub switching_enabled: bool,
    pub switching_disabled_reason: Option<String>,
    #[serde(flatten)]
    pub text: view::ProviderText,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub providers: Vec<ProviderState>,
    pub incomplete: Vec<SwitchRecord>,
    pub credential_store: SecretStoreStatus,
    pub generated_at: String,
    pub activity_marker: String,
    pub stats: view::OverviewStats,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct SwitchPlan {
    pub provider: ProviderId,
    pub account_id: String,
    pub label: String,
    pub mode: SwitchMode,
    pub already_active: bool,
    pub blocked: Option<String>,
    pub requires_confirmation: bool,
    pub restart_apps: Vec<String>,
    pub running_sessions: usize,
    pub warnings: Vec<String>,
    pub needs_confirm: bool,
    pub title: String,
    pub description: String,
    pub confirm_label: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchOptions {
    pub confirm_restart: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct SwitchOutcome {
    pub operation_id: Option<String>,
    pub provider: ProviderId,
    pub account_id: String,
    pub status: String,
    pub mode: SwitchMode,
    pub message: String,
    pub verification: Option<String>,
    pub restarted: Vec<String>,
    pub running_sessions: usize,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub provider: ProviderId,
    pub account_id: String,
    pub stage: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export_to = "bindings.ts")]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryAction {
    RestorePrevious,
    KeepCurrent,
}

fn display_name(p: ProcessInfo) -> String {
    p.path
        .as_deref()
        .and_then(process::app_bundle)
        .and_then(|b| b.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or(p.name)
}

impl Engine {
    pub fn open(cfg: EngineConfig) -> Result<Self> {
        std::fs::create_dir_all(&cfg.data_dir)
            .map_err(|e| Error::io(format!("Cannot create {}", cfg.data_dir.display()), e))?;
        altflip_core::fsutil::restrict_dir(&cfg.data_dir);
        let store = Store::open(&cfg.data_dir.join("database").join("manager.db"))?;
        let vault = Vault::open(&cfg.data_dir.join("vault"), cfg.secrets.clone())?;
        let temp_dir = cfg.data_dir.join("tmp");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let ctx = Ctx { home: cfg.home, io: ResourceIo::new(cfg.secrets.clone()), temp_dir };
        Ok(Self::assemble(store, vault, cfg.secrets, altflip_providers::registry(ctx), cfg.data_dir))
    }

    pub fn assemble(
        store: Store,
        vault: Vault,
        secrets: Arc<dyn SecretStore>,
        providers: Vec<Arc<dyn Provider>>,
        data_dir: PathBuf,
    ) -> Self {
        let locks = providers.iter().map(|p| (p.id(), Mutex::new(()))).collect();
        Engine {
            store: Mutex::new(store),
            vault,
            secrets,
            providers,
            locks,
            logins: Mutex::new(HashMap::new()),
            usage_errors: Mutex::new(HashMap::new()),
            last_live: Mutex::new(HashMap::new()),
            live_cache: Mutex::new(HashMap::new()),
            usage_lock: Mutex::new(()),
            data_dir,
        }
    }

    fn db(&self) -> MutexGuard<'_, Store> {
        self.store.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn provider(&self, id: ProviderId) -> Result<Arc<dyn Provider>> {
        self.providers
            .iter()
            .find(|p| p.id() == id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("{id} is not available")))
    }

    fn account_ref(id: &str) -> String {
        format!("accounts/{id}")
    }

    fn load_snapshot(&self, acct: &AccountRow) -> Result<Snapshot> {
        let raw = self.vault.open_ref(&acct.profile_reference)?;
        let snap = Snapshot::from_bytes(&raw)?;
        if snap.provider != acct.provider || snap.identity.key != acct.account_identifier {
            return Err(Error::Credential(format!("The saved data for {} does not match its account", acct.label)));
        }
        Ok(snap)
    }

    fn seal_snapshot(&self, reference: &str, snap: &Snapshot) -> Result<()> {
        self.vault.seal(reference, &snap.to_bytes()?)
    }

    fn raw_capture(&self, p: &Arc<dyn Provider>) -> Result<Snapshot> {
        let mut resources = Vec::new();
        for (loc, role) in p.locators() {
            resources.push(p.ctx().io.capture(loc, role)?);
        }
        Ok(Snapshot::new(p.id(), Identity::default(), resources))
    }

    pub fn settings(&self) -> Result<Settings> {
        settings::load(&self.db())
    }

    pub fn set_setting(&self, key: &str, value: serde_json::Value) -> Result<Settings> {
        settings::validate(key, &value)?;
        let db = self.db();
        db.set_setting(key, &value)?;
        settings::load(&db)
    }

    pub fn reset_settings(&self) -> Result<Settings> {
        let db = self.db();
        let onboarded = db.setting("onboarded")?;
        db.reset_settings()?;
        if let Some(v) = onboarded {
            db.set_setting("onboarded", &v)?;
        }
        db.log(None, None, "settings", "Settings were reset to defaults")?;
        settings::load(&db)
    }

    fn switching_gate(&self, p: &Arc<dyn Provider>, info: &ProviderInfo) -> Option<String> {
        if !info.installed {
            return Some(format!("{} is not installed", info.name));
        }
        if info.compatibility == Compatibility::Unsupported {
            return Some(info.compatibility_note.clone().unwrap_or_else(|| "This version is not supported".into()));
        }
        if !self.secrets.status().available {
            return Some("The system credential store is unavailable".into());
        }
        if !info.capabilities.account_switching {
            return Some(match p.id() {
                ProviderId::Antigravity => altflip_core::ANTIGRAVITY_USAGE_ONLY.into(),
                _ => format!("{} supports usage only", info.name),
            });
        }
        None
    }

    fn sync_live(&self, p: &Arc<dyn Provider>) -> Result<Option<LiveAccount>> {
        let Some(live) = p.capture()? else { return Ok(None) };
        let health = p.health(&live);
        let saved = self.db().account_by_identity(p.id(), &live.identity.key)?;
        if let Some(acct) = &saved {
            let changed = self.load_snapshot(acct).map(|s| !s.same_primary_data(&live)).unwrap_or(true);
            if changed && health != AuthHealth::NeedsReauth {
                self.seal_snapshot(&acct.profile_reference, &live)?;
            }
            self.db().refresh_identity(&acct.id, &live.identity, health)?;
        }
        Ok(Some(LiveAccount { identity: live.identity.clone(), health, saved_account_id: saved.map(|a| a.id) }))
    }

    fn reconcile_active(&self, provider: ProviderId, live: Option<&LiveAccount>) -> Result<()> {
        let db = self.db();
        let active = db.active_for(provider)?;
        match (live, active) {
            (Some(l), Some(a)) => match &l.saved_account_id {
                Some(id) if *id == a.account_id => {
                    if a.activation_state == "drifted" || a.activation_state == "signed-out" {
                        db.set_activation_state(provider, "verified")?;
                    }
                }
                Some(id) => {
                    let label = db.account(id).map(|r| r.label).unwrap_or_default();
                    db.set_active(provider, id, "external", true)?;
                    db.log(
                        Some(provider),
                        Some(id),
                        "external",
                        &format!("{provider} switched to {label} outside AltFlip"),
                    )?;
                }
                None if a.activation_state != "drifted" => {
                    db.set_activation_state(provider, "drifted")?;
                    db.log(
                        Some(provider),
                        None,
                        "drift",
                        &format!("{provider} is signed in to {}, which is not saved in AltFlip", l.identity.short()),
                    )?;
                }
                None => {}
            },
            (Some(l), None) => {
                if let Some(id) = &l.saved_account_id {
                    db.set_active(provider, id, "verified", true)?;
                }
            }
            (None, Some(a)) if a.activation_state != "signed-out" => {
                db.set_activation_state(provider, "signed-out")?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn overview(&self, refresh_live: bool) -> Result<Overview> {
        let mut states = Vec::new();
        for p in &self.providers {
            let info = p.info();
            let compat = serde_json::to_value(info.compatibility)?;
            let _ = self.db().update_provider(p.id(), info.version.as_deref(), compat.as_str().unwrap_or("unknown"));
            let (live, live_error) = if info.installed && refresh_live {
                let fresh = match self.sync_live(p) {
                    Ok(l) => (l, None),
                    Err(e) => (None, Some(e.to_string())),
                };
                self.live_cache.lock().unwrap_or_else(|e| e.into_inner()).insert(p.id(), fresh.clone());
                fresh
            } else if info.installed {
                self.live_cache.lock().unwrap_or_else(|e| e.into_inner()).get(&p.id()).cloned().unwrap_or((None, None))
            } else {
                (None, None)
            };
            if refresh_live && live_error.is_none() && info.installed {
                self.reconcile_active(p.id(), live.as_ref())?;
            }
            let gate = self.switching_gate(p, &info);
            states.push((info, live, live_error, gate));
        }
        let mut views: HashMap<ProviderId, Vec<AccountView>> = HashMap::new();
        for v in self.account_views(None)? {
            views.entry(v.row.provider).or_default().push(v);
        }
        let db = self.db();
        let active = db.active()?;
        let out: Vec<ProviderState> = states
            .into_iter()
            .map(|(info, live, live_error, gate)| {
                let a = active.iter().find(|a| a.provider == info.id);
                let accounts = views.remove(&info.id).unwrap_or_default();
                let text = view::provider_text(
                    &info,
                    &accounts,
                    a.map(|a| a.account_id.as_str()),
                    a.map(|a| a.activation_state.as_str()),
                    live.as_ref(),
                    live_error.as_deref(),
                );
                ProviderState {
                    accounts,
                    active_account_id: a.map(|a| a.account_id.clone()),
                    activation_state: a.map(|a| a.activation_state.clone()),
                    live,
                    live_error,
                    switching_enabled: gate.is_none(),
                    switching_disabled_reason: gate,
                    info,
                    text,
                }
            })
            .collect();
        let incomplete = db.incomplete_switches()?;
        let activity_marker = db.activity_marker()?;
        let inputs: Vec<view::StatInput> = out
            .iter()
            .map(|p| view::StatInput {
                installed: p.info.installed,
                activation_state: p.activation_state.as_deref(),
                has_active: p.active_account_id.is_some(),
                accounts: &p.accounts,
            })
            .collect();
        let stats = view::overview_stats(&inputs, incomplete.len(), chrono::Utc::now());
        drop(inputs);
        Ok(Overview {
            providers: out,
            incomplete,
            credential_store: self.secrets.status(),
            generated_at: now_rfc3339(),
            activity_marker,
            stats,
        })
    }

    pub fn account_views(&self, provider: Option<ProviderId>) -> Result<Vec<AccountView>> {
        let (active, usage, rows) = {
            let db = self.db();
            (db.active()?, db.latest_usage()?, db.accounts(provider)?)
        };
        let errors = self.usage_errors.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let mut by_account: HashMap<String, Vec<UsageRow>> = HashMap::new();
        for u in usage {
            by_account.entry(u.account_id.clone()).or_default().push(u);
        }
        let now = chrono::Utc::now();
        Ok(rows
            .into_iter()
            .map(|row| {
                let a = active.iter().find(|a| a.account_id == row.id);
                let usage = by_account.remove(&row.id).unwrap_or_default();
                let err = errors.get(&row.id).cloned();
                view::account_view(row, a, usage, err, now)
            })
            .collect())
    }

    pub fn current(&self, provider: ProviderId) -> Result<Option<LiveAccount>> {
        let p = self.provider(provider)?;
        let Some(live) = p.capture()? else { return Ok(None) };
        let saved = self.db().account_by_identity(provider, &live.identity.key)?.map(|a| a.id);
        Ok(Some(LiveAccount { health: p.health(&live), identity: live.identity, saved_account_id: saved }))
    }

    fn default_label(&self, provider: ProviderId, identity: &Identity) -> String {
        identity
            .email
            .as_deref()
            .and_then(|e| e.split('@').next())
            .map(str::to_string)
            .or_else(|| identity.display_name.clone())
            .unwrap_or_else(|| {
                let n = self.db().accounts(Some(provider)).map(|a| a.len() + 1).unwrap_or(1);
                format!("{provider} account {n}")
            })
    }

    fn upsert_guard(snap: &Snapshot, health: AuthHealth) -> Result<()> {
        if health == AuthHealth::NeedsReauth {
            return Err(Error::Credential(format!(
                "{}'s current sign-in is unreadable or incomplete. {}",
                snap.provider,
                sign_in_hint(snap.provider)
            )));
        }
        Ok(())
    }

    fn upsert(
        &self,
        snap: &Snapshot,
        label: Option<String>,
        category: Option<String>,
        health: AuthHealth,
    ) -> Result<(AccountRow, bool)> {
        Self::upsert_guard(snap, health)?;
        let provider = snap.provider;
        let label = label.filter(|l| !l.trim().is_empty());
        let existing = self.db().account_by_identity(provider, &snap.identity.key)?;
        if let Some(acct) = existing {
            self.seal_snapshot(&acct.profile_reference, snap)?;
            let db = self.db();
            db.refresh_identity(&acct.id, &snap.identity, health)?;
            if label.is_some() || category.is_some() {
                db.patch_account(
                    &acct.id,
                    &AccountPatch { label, category: category.map(Some), ..Default::default() },
                )?;
            }
            return Ok((db.account(&acct.id)?, false));
        }
        let id = new_id();
        let reference = Self::account_ref(&id);
        self.seal_snapshot(&reference, snap)?;
        let label = label.unwrap_or_else(|| self.default_label(provider, &snap.identity));
        let row = self.db().insert_account(
            &id,
            &NewAccount {
                provider,
                label,
                category,
                identity: snap.identity.clone(),
                profile_reference: reference.clone(),
                health,
            },
        );
        match row {
            Ok(r) => Ok((r, true)),
            Err(e) => {
                let _ = self.vault.remove(&reference);
                Err(e)
            }
        }
    }

    pub fn save_current(
        &self,
        provider: ProviderId,
        label: Option<String>,
        category: Option<String>,
    ) -> Result<AccountRow> {
        let p = self.provider(provider)?;
        let _guard = self.lock(provider)?;
        let live = p
            .capture()?
            .ok_or_else(|| Error::NotFound(format!("{provider} is not signed in. Sign in with {provider} first.")))?;
        let health = p.health(&live);
        let (row, created) = self.upsert(&live, label, category, health)?;
        let db = self.db();
        db.set_active(provider, &row.id, "verified", true)?;
        db.log(
            Some(provider),
            Some(&row.id),
            if created { "added" } else { "updated" },
            &format!("{} {} ({})", if created { "Saved" } else { "Updated" }, row.label, live.identity.short()),
        )?;
        Ok(row)
    }

    pub fn login_new(
        &self,
        provider: ProviderId,
        login_id: &str,
        label: Option<String>,
        category: Option<String>,
        out: &(dyn Fn(&str) + Send + Sync),
    ) -> Result<AccountRow> {
        let p = self.provider(provider)?;
        let replaces = p.capabilities().login_replaces_current;
        let _guard = if replaces { Some(self.lock(provider)?) } else { None };
        if replaces {
            if let Some(live) = p.capture()? {
                let health = p.health(&live);
                let (prev, created) = self.upsert(&live, None, None, health)?;
                if created {
                    self.db().log(
                        Some(provider),
                        Some(&prev.id),
                        "added",
                        &format!("Saved {} before signing in another account", prev.label),
                    )?;
                }
                out(&format!("Saved the current account ({}) so you can switch back to it.", live.identity.short()));
            }
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.logins.lock().unwrap_or_else(|e| e.into_inner()).insert(login_id.to_string(), cancel.clone());
        let res = p.login(&cancel, out);
        self.logins.lock().unwrap_or_else(|e| e.into_inner()).remove(login_id);
        let snap = res?;
        let health = p.health(&snap);
        let (row, created) = self.upsert(&snap, label, category, health)?;
        if replaces {
            self.db().set_active(provider, &row.id, "verified", true)?;
        }
        self.db().log(
            Some(provider),
            Some(&row.id),
            if created { "added" } else { "updated" },
            &format!("Signed in {} ({})", row.label, snap.identity.short()),
        )?;
        Ok(row)
    }

    pub fn cancel_login(&self, login_id: &str) {
        if let Some(c) = self.logins.lock().unwrap_or_else(|e| e.into_inner()).get(login_id) {
            c.store(true, Ordering::SeqCst);
        }
    }

    pub fn update_account(&self, id: &str, patch: AccountPatch) -> Result<AccountRow> {
        let db = self.db();
        let before = db.account(id)?;
        let after = db.patch_account(id, &patch)?;
        if before.label != after.label {
            db.log(Some(after.provider), Some(id), "renamed", &format!("Renamed {} to {}", before.label, after.label))?;
        }
        Ok(after)
    }

    pub fn reorder(&self, provider: ProviderId, ids: &[String]) -> Result<()> {
        self.db().reorder(provider, ids)
    }

    pub fn remove_account(&self, id: &str) -> Result<()> {
        let acct = self.db().account(id)?;
        let _guard = self.lock(acct.provider)?;
        let db = self.db();
        db.delete_account(id)?;
        self.vault.remove(&acct.profile_reference)?;
        db.log(
            Some(acct.provider),
            None,
            "removed",
            &format!("Removed {} from AltFlip. The {} sign-in itself was not changed.", acct.label, acct.provider),
        )?;
        Ok(())
    }

    fn lock(&self, p: ProviderId) -> Result<MutexGuard<'_, ()>> {
        self.locks
            .get(&p)
            .ok_or_else(|| Error::NotFound(p.to_string()))?
            .try_lock()
            .map_err(|_| Error::Busy(p.to_string()))
    }

    pub fn plan_switch(&self, account_id: &str) -> Result<SwitchPlan> {
        let acct = self.db().account(account_id)?;
        let p = self.provider(acct.provider)?;
        let info = p.info();
        let mut blocked = self.switching_gate(&p, &info);
        if blocked.is_none() {
            if let Err(e) = self.load_snapshot(&acct) {
                blocked = Some(e.to_string());
            }
        }
        let live_key = p.capture().ok().flatten().map(|s| s.identity.key);
        let already_active = live_key.as_deref() == Some(acct.account_identifier.as_str());
        let restart: Vec<String> = p.restart_targets(&info.processes).into_iter().map(display_name).collect();
        let running_sessions = if info.switch_mode == SwitchMode::Immediate {
            0
        } else {
            info.processes.iter().filter(|p| p.kind == ProcessKind::Cli).count()
        };
        let mut warnings = Vec::new();
        if running_sessions > 0 {
            warnings.push(format!(
                "{running_sessions} running {} session{} will keep the previous account until restarted, and may change the sign-in back if it refreshes its credentials.",
                acct.provider,
                if running_sessions == 1 { "" } else { "s" }
            ));
        }
        if !restart.is_empty() {
            warnings.push(format!(
                "{} will be closed and reopened. Save your work first; open agent conversations will not move to the new account.",
                restart.join(" and ")
            ));
        }
        if matches!(acct.health, AuthHealth::Expired | AuthHealth::NeedsReauth) {
            warnings.push(
                "This account's saved sign-in may have expired. You may need to sign in again after switching.".into(),
            );
        }
        let warn = self.settings().map(|s| s.warn_running_sessions).unwrap_or(true);
        let requires_confirmation = !restart.is_empty();
        let needs_confirm = requires_confirmation || (warn && !warnings.is_empty());
        let title = if requires_confirmation {
            format!("Restart {}?", restart.join(" and "))
        } else {
            format!("Switch {} to {}?", info.name, acct.label)
        };
        Ok(SwitchPlan {
            provider: acct.provider,
            account_id: acct.id,
            label: acct.label,
            mode: info.switch_mode,
            already_active,
            blocked,
            requires_confirmation,
            restart_apps: restart,
            running_sessions,
            warnings,
            needs_confirm,
            title,
            description: view::mode_text(info.switch_mode).into(),
            confirm_label: if requires_confirmation { "Restart and switch".into() } else { "Switch".into() },
        })
    }

    pub fn switch(
        &self,
        account_id: &str,
        opts: SwitchOptions,
        progress: &(dyn Fn(Progress) + Send + Sync),
    ) -> Result<SwitchOutcome> {
        let acct = self.db().account(account_id)?;
        let provider = acct.provider;
        let p = self.provider(provider)?;
        let _guard = self.lock(provider)?;
        let say = |stage: &str, message: String| {
            progress(Progress { provider, account_id: acct.id.clone(), stage: stage.into(), message })
        };
        say("validating", format!("Checking {}", acct.label));
        let info = p.info();
        if let Some(reason) = self.switching_gate(&p, &info) {
            return Err(Error::Incompatible(reason));
        }
        let target = self.load_snapshot(&acct)?;
        let live = p.capture()?;
        if let Some(l) = live.as_ref().filter(|l| l.identity.key == target.identity.key) {
            self.seal_snapshot(&acct.profile_reference, l)?;
            let db = self.db();
            db.set_active(provider, &acct.id, "verified", true)?;
            db.mark_used(&acct.id)?;
            return Ok(SwitchOutcome {
                operation_id: None,
                provider,
                account_id: acct.id.clone(),
                status: "already-active".into(),
                mode: info.switch_mode,
                message: format!("{provider} is already using {}", acct.label),
                verification: None,
                restarted: vec![],
                running_sessions: 0,
                detail: None,
            });
        }
        let restart = p.restart_targets(&info.processes);
        if !restart.is_empty() && !opts.confirm_restart {
            return Err(Error::ConfirmationRequired(format!(
                "{} must restart to finish switching. Confirm to continue.",
                restart.into_iter().map(display_name).collect::<Vec<_>>().join(" and ")
            )));
        }
        let running_sessions = if info.switch_mode == SwitchMode::Immediate {
            0
        } else {
            info.processes.iter().filter(|p| p.kind == ProcessKind::Cli).count()
        };

        let from = live.as_ref().and_then(|l| self.db().account_by_identity(provider, &l.identity.key).ok().flatten());
        if let (Some(l), Some(f)) = (&live, &from) {
            self.seal_snapshot(&f.profile_reference, l)?;
        }
        let op = new_id();
        let started = now_rfc3339();
        let previous = self.raw_capture(&p)?;
        let recovery_ref = format!("recovery/{op}");
        self.seal_snapshot(&recovery_ref, &previous)?;
        {
            let db = self.db();
            db.begin_switch(&SwitchRecord {
                id: op.clone(),
                provider,
                from_account_id: from.as_ref().map(|f| f.id.clone()),
                to_account_id: Some(acct.id.clone()),
                from_label: from
                    .as_ref()
                    .map(|f| f.label.clone())
                    .or_else(|| live.as_ref().map(|l| l.identity.short())),
                to_label: Some(acct.label.clone()),
                status: "started".into(),
                switch_mode: serde_json::to_value(info.switch_mode)?.as_str().unwrap_or_default().into(),
                error_code: None,
                message: None,
                started_at: started.clone(),
                completed_at: None,
            })?;
            db.add_recovery(&RecoveryRow {
                id: new_id(),
                provider,
                operation_id: op.clone(),
                snapshot_reference: recovery_ref,
                created_at: started,
                status: "pending".into(),
            })?;
        }

        let mut closed: Vec<PathBuf> = Vec::new();
        if !restart.is_empty() {
            let names = restart.iter().cloned().map(display_name).collect::<Vec<_>>().join(", ");
            say("closing", format!("Closing {names}"));
            for t in &restart {
                if let Err(e) = process::request_quit(t) {
                    let aborted = self.abort_before_apply(&op, &e);
                    self.reopen(&closed);
                    aborted?;
                    return Err(e);
                }
                if let Some(path) = &t.path {
                    closed.push(path.clone());
                }
            }
            let pids: Vec<u32> = p
                .processes()
                .into_iter()
                .filter(|x| matches!(x.kind, ProcessKind::App | ProcessKind::Ide | ProcessKind::Helper))
                .map(|x| x.pid)
                .collect();
            if !process::wait_exit(&pids, Duration::from_secs(25)) {
                let e = Error::Platform(format!(
                    "{} did not close. Nothing was changed. Close it yourself (it may be asking to save work) and try again.",
                    info.name
                ));
                let aborted = self.abort_before_apply(&op, &e);
                self.reopen(&closed);
                aborted?;
                return Err(e);
            }
        }

        if let Err(e) = self.db().set_switch_status(&op, "applying") {
            let _ = self.abort_before_apply(&op, &e);
            self.reopen(&closed);
            return Err(e);
        }
        say("activating", format!("Activating {}", acct.label));
        let result = p.apply(&target).and_then(|_| {
            say("verifying", "Verifying the active account".into());
            p.verify(&target)
        });

        let outcome = (|| match result {
            Ok(verification) => {
                let state = if running_sessions > 0 { "new-sessions" } else { "verified" };
                let db = self.db();
                db.set_active(provider, &acct.id, state, true)?;
                db.mark_used(&acct.id)?;
                db.finish_switch(&op, "succeeded", None, Some(&verification))?;
                db.set_recovery_status(&op, "available")?;
                db.log(Some(provider), Some(&acct.id), "switched", &format!("{provider} → {}", acct.label))?;
                Ok(SwitchOutcome {
                    operation_id: Some(op.clone()),
                    provider,
                    account_id: acct.id.clone(),
                    status: "succeeded".into(),
                    mode: info.switch_mode,
                    message: if running_sessions > 0 {
                        format!("{provider} now uses {} for new sessions", acct.label)
                    } else {
                        format!("{provider} is now using {}", acct.label)
                    },
                    verification: Some(verification),
                    restarted: vec![],
                    running_sessions,
                    detail: None,
                })
            }
            Err(e) => {
                say("rolling-back", "Restoring the previous account".into());
                let restored = p.restore(&previous).and_then(|_| {
                    let now = self.raw_capture(&p)?;
                    if previous.resources.iter().all(|r| now.data_for(&r.locator) == r.data.as_deref()) {
                        Ok(())
                    } else {
                        Err(Error::Verification("Previous credentials could not be restored exactly".into()))
                    }
                });
                let db = self.db();
                match restored {
                    Ok(()) => {
                        db.finish_switch(&op, "rolled-back", Some(e.code()), Some(&e.to_string()))?;
                        db.set_recovery_status(&op, "available")?;
                        db.set_account_error(&acct.id, Some(&e.to_string()))?;
                        db.log(
                            Some(provider),
                            Some(&acct.id),
                            "rolled-back",
                            &format!("Could not activate {}. The previous account was restored.", acct.label),
                        )?;
                        Err(Error::Verification(format!(
                            "Could not activate {}: {e}. Your previous account has been restored.",
                            acct.label
                        )))
                    }
                    Err(re) => {
                        db.finish_switch(
                            &op,
                            "needs-recovery",
                            Some(re.code()),
                            Some(&format!("{e}; rollback: {re}")),
                        )?;
                        db.log(
                            Some(provider),
                            Some(&acct.id),
                            "needs-recovery",
                            &format!("Switch to {} failed and automatic rollback did not complete", acct.label),
                        )?;
                        Err(Error::Verification(format!(
                            "Could not activate {}: {e}. Automatic rollback also failed ({re}). Open Activity to restore the previous account.",
                            acct.label
                        )))
                    }
                }
            }
        })();

        if !closed.is_empty() {
            say("reopening", "Reopening apps".into());
        }
        let reopened = self.reopen(&closed);
        self.prune_recoveries(10);
        outcome.map(|mut o| {
            let mut detail = o.verification.clone().unwrap_or_default();
            if !reopened.is_empty() {
                detail.push_str(&format!(" Reopened {}.", reopened.join(", ")));
            }
            o.detail = (!detail.trim().is_empty()).then(|| detail.trim().to_string());
            o.restarted = reopened;
            o
        })
    }

    fn abort_before_apply(&self, op: &str, e: &Error) -> Result<()> {
        let db = self.db();
        db.finish_switch(op, "cancelled", Some(e.code()), Some(&e.to_string()))?;
        db.set_recovery_status(op, "discarded")?;
        Ok(())
    }

    fn reopen(&self, paths: &[PathBuf]) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for p in paths {
            let key = process::app_bundle(p).unwrap_or_else(|| p.clone());
            if seen.insert(key.clone()) && process::launch(p).is_ok() {
                out.push(key.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
            }
        }
        out
    }

    fn prune_recoveries(&self, keep: usize) {
        let db = self.db();
        let Ok(rows) = db.recoveries(None) else { return };
        for (i, r) in rows.iter().enumerate() {
            if (i >= keep && r.status != "pending") || r.status == "discarded" {
                let _ = self.vault.remove(&r.snapshot_reference);
                let _ = db.delete_recovery(&r.id);
            }
        }
    }

    pub fn incomplete(&self) -> Result<Vec<SwitchRecord>> {
        self.db().incomplete_switches()
    }

    pub fn recover_on_launch(&self) -> Result<Vec<SwitchRecord>> {
        let db = self.db();
        let mut pending = Vec::new();
        for r in db.incomplete_switches()? {
            if r.status == "started" {
                db.finish_switch(
                    &r.id,
                    "abandoned",
                    Some("interrupted"),
                    Some("AltFlip stopped before any change was made"),
                )?;
                db.set_recovery_status(&r.id, "discarded")?;
            } else {
                db.log(
                    Some(r.provider),
                    r.to_account_id.as_deref(),
                    "interrupted",
                    "A switch was interrupted. Choose how to recover.",
                )?;
                pending.push(r);
            }
        }
        Ok(pending)
    }

    pub fn resolve_incomplete(&self, operation_id: &str, action: RecoveryAction) -> Result<()> {
        let record = self.db().switch_record(operation_id)?;
        let p = self.provider(record.provider)?;
        {
            let _guard = self.lock(record.provider)?;
            match action {
                RecoveryAction::RestorePrevious => {
                    let rec = self
                        .db()
                        .recoveries(None)?
                        .into_iter()
                        .find(|r| r.operation_id == operation_id && r.status != "discarded")
                        .ok_or_else(|| Error::NotFound("No recovery snapshot exists for that switch".into()))?;
                    let snap = Snapshot::from_bytes(&self.vault.open_ref(&rec.snapshot_reference)?)?;
                    p.restore(&snap)?;
                    let db = self.db();
                    db.finish_switch(operation_id, "recovered", None, Some("Previous account restored"))?;
                    db.set_recovery_status(operation_id, "used")?;
                    db.log(
                        Some(record.provider),
                        None,
                        "recovered",
                        &format!("Restored {}'s previous sign-in", record.provider),
                    )?;
                }
                RecoveryAction::KeepCurrent => {
                    let db = self.db();
                    db.finish_switch(operation_id, "resolved", None, Some("Kept the current sign-in"))?;
                    db.log(
                        Some(record.provider),
                        None,
                        "recovered",
                        &format!("Kept {}'s current sign-in", record.provider),
                    )?;
                }
            }
        }
        let live = self.sync_live(&p)?;
        self.reconcile_active(record.provider, live.as_ref())
    }

    pub fn undo_switch(&self, operation_id: &str) -> Result<()> {
        let record = self.db().switch_record(operation_id)?;
        if record.status != "succeeded" {
            return Err(Error::Invalid("Only a completed switch can be undone".into()));
        }
        let latest = self
            .db()
            .switch_history(50)?
            .into_iter()
            .find(|s| s.provider == record.provider && s.status == "succeeded");
        if latest.map(|l| l.id) != Some(record.id.clone()) {
            return Err(Error::Invalid("Only the most recent switch for a provider can be undone".into()));
        }
        self.resolve_incomplete(operation_id, RecoveryAction::RestorePrevious)
    }

    pub fn history(&self, limit: i64) -> Result<Vec<SwitchRecord>> {
        self.db().switch_history(limit)
    }

    pub fn history_views(&self, limit: i64) -> Result<Vec<SwitchView>> {
        let all = self.db().switch_history(limit.max(100))?;
        let undoable = view::undoable_ids(&all);
        let now = chrono::Utc::now();
        Ok(all
            .into_iter()
            .take(limit.max(0) as usize)
            .map(|r| {
                let u = undoable.contains(&r.id);
                view::switch_view(r, u, now)
            })
            .collect())
    }

    pub fn activity_feed(&self, q: &FeedQuery) -> Result<Feed> {
        let (history, log, labels) = {
            let db = self.db();
            let labels: HashMap<String, String> =
                db.accounts(None)?.into_iter().map(|a| (a.id, a.label)).collect();
            (db.switch_history(500)?, db.activity(1000)?, labels)
        };
        Ok(view::feed(history, log, |id| labels.get(id).cloned(), q, chrono::Utc::now()))
    }

    pub fn usage_notice(&self) -> Option<Notice> {
        let counts = self.db().accounts(None).unwrap_or_default();
        let infos: Vec<ProviderInfo> = self.providers.iter().map(|p| p.info()).collect();
        let states: Vec<(&ProviderInfo, usize)> =
            infos.iter().map(|i| (i, counts.iter().filter(|a| a.provider == i.id).count())).collect();
        view::unsupported_usage_notice(&states)
    }

    pub fn move_account(&self, id: &str, delta: i64) -> Result<()> {
        let acct = self.db().account(id)?;
        let mut ids: Vec<String> = self.db().accounts(Some(acct.provider))?.into_iter().map(|a| a.id).collect();
        let Some(i) = ids.iter().position(|x| x == id) else { return Ok(()) };
        let j = i as i64 + delta;
        if j < 0 || j >= ids.len() as i64 {
            return Ok(());
        }
        ids.swap(i, j as usize);
        self.db().reorder(acct.provider, &ids)
    }

    pub fn activity(&self, limit: i64) -> Result<Vec<ActivityRow>> {
        self.db().activity(limit)
    }

    pub fn refresh_usage(&self) -> Result<Vec<UsageRow>> {
        self.refresh_usage_with(false, None)
    }

    pub fn refresh_usage_with(&self, force: bool, only: Option<ProviderId>) -> Result<Vec<UsageRow>> {
        let _running = if force {
            self.usage_lock.lock().unwrap_or_else(|e| e.into_inner())
        } else {
            match self.usage_lock.try_lock() {
                Ok(g) => g,
                Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner(),
                Err(std::sync::TryLockError::WouldBlock) => return self.db().latest_usage(),
            }
        };
        let settings = self.settings()?;
        if !settings.usage_monitoring {
            return self.db().latest_usage();
        }
        let live_on = settings.live_usage;
        for p in self.providers.iter().filter(|p| only.is_none_or(|id| id == p.id())) {
            let mut readings = p.usage();
            let mut stale_sources: Vec<(Option<String>, &str)> = Vec::new();
            match p.id() {
                ProviderId::Claude | ProviderId::Codex if live_on => {
                    let local = if p.id() == ProviderId::Claude { CLAUDE_ESTIMATE_SOURCE } else { CODEX_LOG_SOURCE };
                    readings.retain(|r| r.source != local);
                    stale_sources.push((None, local));
                    readings.extend(self.live_usage(p, force, &HashSet::new()));
                }
                ProviderId::Antigravity => {
                    use altflip_providers::antigravity::{live_usage, LIVE_SOURCE, LOCAL_SOURCE};
                    let mut live = live_usage();
                    if live_on {
                        let covered: HashSet<String> = live.iter().filter_map(|r| r.account_key.clone()).collect();
                        live.extend(self.live_usage(p, force, &covered));
                    }
                    let keys: HashSet<Option<String>> = live.iter().map(|r| r.account_key.clone()).collect();
                    let had_live: HashSet<String> = self
                        .db()
                        .latest_usage()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|u| u.reading.source == LIVE_SOURCE)
                        .map(|u| u.account_id)
                        .collect();
                    let accounts = self.db().accounts(Some(p.id())).unwrap_or_default();
                    let account_of = |key: &Option<String>| {
                        key.as_ref()
                            .and_then(|k| accounts.iter().find(|a| &a.account_identifier == k))
                            .map(|a| a.id.clone())
                    };
                    readings.retain(|r| {
                        !keys.contains(&r.account_key)
                            && account_of(&r.account_key).is_none_or(|id| !had_live.contains(&id))
                    });
                    for key in &keys {
                        if let Some(id) = account_of(key) {
                            stale_sources.push((Some(id.clone()), LOCAL_SOURCE));
                            stale_sources.push((Some(id), LIVE_SOURCE));
                        }
                    }
                    readings.extend(live);
                }
                _ => {}
            }
            for (account, source) in &stale_sources {
                let _ = self.db().drop_usage_source(p.id(), account.as_deref(), source);
            }
            if readings.is_empty() {
                continue;
            }
            let db = self.db();
            let active = db
                .active_for(p.id())?
                .filter(|a| matches!(a.activation_state.as_str(), "verified" | "new-sessions" | "external"))
                .map(|a| a.account_id);
            let mut switches: Vec<SwitchRecord> = db
                .switch_history(1000)?
                .into_iter()
                .filter(|s| s.provider == p.id() && matches!(s.status.as_str(), "succeeded" | "recovered"))
                .collect();
            switches.sort_by(|a, b| a.completed_at.cmp(&b.completed_at));
            let accounts = db.accounts(Some(p.id()))?;
            let account_at = |ts: &str| -> Option<String> {
                match switches.iter().rev().find(|s| s.completed_at.as_deref().is_some_and(|c| c <= ts)) {
                    Some(s) if s.status == "succeeded" => s.to_account_id.clone(),
                    Some(_) => None,
                    None => match switches.first() {
                        Some(first) => first.from_account_id.clone(),
                        None => active.clone(),
                    },
                }
            };
            let mut batch = Vec::with_capacity(readings.len());
            for r in &readings {
                let target = match &r.account_key {
                    Some(key) if key.starts_with("claude-org:") => {
                        let suffix = format!(":{}", &key["claude-org:".len()..]);
                        let hits: Vec<_> =
                            accounts.iter().filter(|a| a.account_identifier.ends_with(&suffix)).collect();
                        match hits.as_slice() {
                            [one] => Some(one.id.clone()),
                            _ => None,
                        }
                    }
                    Some(key) => accounts.iter().find(|a| &a.account_identifier == key).map(|a| a.id.clone()),
                    None => account_at(&r.observed_at),
                };
                if let Some(id) = target {
                    batch.push((id, r));
                }
            }
            db.add_usage_many(p.id(), &batch)?;
        }
        let _ = self.db().prune_usage(7);
        self.db().latest_usage()
    }

    fn note_usage_error(&self, row: &AccountRow, err: Option<String>) {
        let fresh = {
            let mut map = self.usage_errors.lock().unwrap_or_else(|e| e.into_inner());
            match err {
                Some(e) if map.get(&row.id) != Some(&e) => {
                    map.insert(row.id.clone(), e.clone());
                    Some(e)
                }
                Some(_) => None,
                None => {
                    map.remove(&row.id);
                    None
                }
            }
        };
        if let Some(e) = fresh {
            let _ = self.db().log(
                Some(row.provider),
                Some(&row.id),
                "usage",
                &format!("Usage for {} unavailable: {e}", row.label),
            );
        }
    }

    fn claude_snapshot_for_usage(&self, p: &Arc<dyn Provider>, row: &AccountRow) -> Result<Snapshot> {
        use altflip_providers::claude;
        let live = p.capture().ok().flatten().filter(|l| l.identity.key == row.account_identifier);
        let snap = match &live {
            Some(l) => l.clone(),
            None => self.load_snapshot(row)?,
        };
        if p.health(&snap) == AuthHealth::NeedsReauth || claude::creds_locator(&snap).is_none() {
            return Err(Error::Credential(CLAUDE_SIGNED_OUT.into()));
        }
        if !claude::needs_renewal(&snap) {
            return Ok(snap);
        }
        let _guard = self.lock(ProviderId::Claude)?;
        let renewed = claude::renew(&snap)?;
        if live.is_some() {
            let now = p.capture()?.ok_or_else(|| Error::Credential("Claude Code signed out".into()))?;
            if now.identity.key != row.account_identifier || !now.same_primary_data(&snap) {
                return Err(Error::Verification("Claude Code changed its login while AltFlip was renewing it".into()));
            }
            let loc =
                claude::creds_locator(&renewed).ok_or_else(|| Error::Credential("No saved Claude login".into()))?;
            let data = renewed.data_for(&loc).ok_or_else(|| Error::Credential("No saved Claude login".into()))?;
            p.ctx().io.write(&loc, data)?;
        }
        self.seal_snapshot(&row.profile_reference, &renewed)?;
        let _ = self.db().log(
            Some(ProviderId::Claude),
            Some(&row.id),
            "renewed",
            &format!("Renewed the Claude login for {}", row.label),
        );
        Ok(renewed)
    }

    fn live_usage(
        &self,
        p: &Arc<dyn Provider>,
        force: bool,
        covered: &HashSet<String>,
    ) -> Vec<altflip_core::UsageReading> {
        let rows = self.db().accounts(Some(p.id())).unwrap_or_default();
        let live = p.capture().ok().flatten();
        let mut out = Vec::new();
        let mut with_reading: Option<HashSet<String>> = None;
        for row in rows {
            if covered.contains(&row.account_identifier) {
                continue;
            }
            {
                let mut last = self.last_live.lock().unwrap_or_else(|e| e.into_inner());
                let every = if matches!(p.id(), ProviderId::Claude | ProviderId::Antigravity) { 120 } else { 60 };
                let min = if force { 15 } else { every };
                if last.get(&row.id).is_some_and(|t| t.elapsed() < Duration::from_secs(min)) {
                    continue;
                }
                last.insert(row.id.clone(), std::time::Instant::now());
            }
            let res = match p.id() {
                ProviderId::Claude => {
                    self.claude_snapshot_for_usage(p, &row).and_then(|s| altflip_providers::claude::exact_usage(&s))
                }
                ProviderId::Codex => {
                    let snap = match live.as_ref().filter(|l| l.identity.key == row.account_identifier) {
                        Some(l) => Ok(l.clone()),
                        None => self.load_snapshot(&row),
                    };
                    snap.and_then(|s| altflip_providers::codex::live_usage(&s))
                }
                ProviderId::Antigravity => {
                    let snap = match live.as_ref().filter(|l| l.identity.key == row.account_identifier) {
                        Some(l) => Ok(l.clone()),
                        None => self.load_snapshot(&row),
                    };
                    snap.and_then(|s| altflip_providers::antigravity_cloud::usage(&s, &p.ctx().home)).map(|q| {
                        if let Some(tier) = q.tier.filter(|t| row.subscription_type.as_deref() != Some(t.as_str())) {
                            let _ = self.db().set_plan(&row.id, &tier);
                        }
                        q.readings
                    })
                }
            };
            match res {
                Ok(mut r) => {
                    for u in r.iter_mut() {
                        u.account_key = Some(row.account_identifier.clone());
                    }
                    out.extend(r);
                    self.note_usage_error(&row, None);
                }
                Err(e) => {
                    let transient = matches!(e, Error::Platform(_) | Error::Busy(_));
                    let has_reading = transient
                        && with_reading
                            .get_or_insert_with(|| {
                                self.db().latest_usage().unwrap_or_default().into_iter().map(|u| u.account_id).collect()
                            })
                            .contains(&row.id);
                    if has_reading {
                        self.note_usage_error(&row, None);
                    } else if altflip_providers::claude::is_throttled(&e) {
                        self.note_usage_error(&row, Some("Waiting for the first reading from Anthropic.".into()));
                    } else {
                        self.note_usage_error(&row, Some(e.to_string()));
                    }
                }
            }
        }
        out
    }

    fn claude_config_dir(&self) -> Result<std::path::PathBuf> {
        Ok(self.provider(ProviderId::Claude)?.ctx().home.join(".claude"))
    }

    pub fn claude_status_line(&self) -> Result<altflip_providers::claude_status::StatusLineState> {
        Ok(altflip_providers::claude_status::state(&self.claude_config_dir()?))
    }

    pub fn set_claude_status_line(&self, on: bool) -> Result<altflip_providers::claude_status::StatusLineState> {
        use altflip_providers::claude_status;
        let dir = self.claude_config_dir()?;
        let st = if on { claude_status::install(&dir)? } else { claude_status::uninstall(&dir)? };
        let msg = if on {
            "Installed the AltFlip status line in Claude Code for live usage"
        } else {
            "Removed the AltFlip status line from Claude Code"
        };
        let _ = self.db().log(Some(ProviderId::Claude), None, "settings", msg);
        Ok(st)
    }

    pub fn account(&self, account_id: &str) -> Result<AccountRow> {
        self.db().account(account_id)
    }

    pub fn usage_error(&self, account_id: &str) -> Option<String> {
        self.usage_errors.lock().ok()?.get(account_id).cloned()
    }

    pub fn export_backup(&self, path: &Path, passphrase: &str) -> Result<usize> {
        backup::export(self, path, passphrase)
    }

    pub fn import_backup(&self, path: &Path, passphrase: &str) -> Result<usize> {
        backup::import(self, path, passphrase)
    }

    pub fn diagnostics(&self) -> Result<serde_json::Value> {
        let overview = self.overview(false)?;
        let db = self.db();
        Ok(serde_json::json!({
            "app": env!("CARGO_PKG_VERSION"),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "database": db.integrity_check()?,
            "credentialStore": self.secrets.status(),
            "providers": overview.providers.iter().map(|p| serde_json::json!({
                "id": p.info.id,
                "installed": p.info.installed,
                "version": p.info.version,
                "compatibility": p.info.compatibility,
                "note": p.info.compatibility_note,
                "switchMode": p.info.switch_mode,
                "accounts": p.accounts.len(),
                "runningProcesses": p.info.processes.len(),
                "switchingEnabled": p.switching_enabled,
            })).collect::<Vec<_>>(),
            "recentSwitches": db.switch_history(20)?.into_iter().map(|s| serde_json::json!({
                "provider": s.provider, "status": s.status, "mode": s.switch_mode, "error": s.error_code, "at": s.started_at
            })).collect::<Vec<_>>(),
        }))
    }
}

#[cfg(test)]
mod tests;
