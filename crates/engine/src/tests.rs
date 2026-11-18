use super::*;
use altflip_core::resource::{Locator, Role};
use altflip_core::secrets::MemorySecrets;
use altflip_core::{Capabilities, Support};
use altflip_providers::Detection;
use std::sync::atomic::AtomicBool;

struct Fake {
    ctx: Ctx,
    file: PathBuf,
    fail_verify: AtomicBool,
    usage_only: bool,
    readings: Mutex<Vec<altflip_core::UsageReading>>,
    broken: AtomicBool,
}

impl Provider for Fake {
    fn id(&self) -> ProviderId {
        if self.usage_only {
            ProviderId::Antigravity
        } else {
            ProviderId::Codex
        }
    }
    fn ctx(&self) -> &Ctx {
        &self.ctx
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            multiple_accounts: true,
            account_switching: !self.usage_only,
            live_switching: false,
            isolated_login: false,
            login_replaces_current: false,
            usage_monitoring: Support::No,
            restart_required: Support::No,
        }
    }
    fn detection(&self) -> Detection {
        Detection {
            installed: true,
            version: Some("1.0.0".into()),
            binary_path: None,
            config_path: None,
            compatibility: Compatibility::Supported,
            note: None,
        }
    }
    fn locators(&self) -> Vec<(Locator, Role)> {
        vec![(Locator::File { path: self.file.clone() }, Role::Primary)]
    }
    fn identify(&self, snap: &Snapshot) -> Result<Option<Identity>> {
        let Some(d) = snap.data_for(&Locator::File { path: self.file.clone() }) else { return Ok(None) };
        let v: serde_json::Value = serde_json::from_slice(d)?;
        let user = v["user"].as_str().unwrap_or_default().to_string();
        Ok(Some(Identity { key: format!("u:{user}"), email: Some(format!("{user}@x.dev")), ..Default::default() }))
    }
    fn health(&self, _snap: &Snapshot) -> AuthHealth {
        if self.broken.load(Ordering::SeqCst) {
            AuthHealth::NeedsReauth
        } else {
            AuthHealth::Ok
        }
    }
    fn processes(&self) -> Vec<ProcessInfo> {
        vec![]
    }
    fn usage(&self) -> Vec<altflip_core::UsageReading> {
        self.readings.lock().unwrap().clone()
    }
    fn verify(&self, expected: &Snapshot) -> Result<String> {
        if self.fail_verify.load(Ordering::SeqCst) {
            return Err(Error::Verification("forced failure".into()));
        }
        let live = self.capture()?.ok_or_else(|| Error::Verification("none".into()))?;
        if live.identity.key == expected.identity.key {
            Ok("ok".into())
        } else {
            Err(Error::Verification("mismatch".into()))
        }
    }
}

struct Harness {
    _dir: tempfile::TempDir,
    engine: Engine,
    fake: Arc<Fake>,
    secrets: Arc<dyn SecretStore>,
    root: PathBuf,
}

fn harness(usage_only: bool) -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let secrets: Arc<dyn SecretStore> = Arc::new(MemorySecrets::default());
    let fake = Arc::new(Fake {
        ctx: Ctx { home: root.clone(), io: ResourceIo::new(secrets.clone()), temp_dir: root.join("tmp") },
        file: root.join("auth.json"),
        fail_verify: AtomicBool::new(false),
        usage_only,
        readings: Mutex::new(vec![]),
        broken: AtomicBool::new(false),
    });
    let engine = build(&root, secrets.clone(), fake.clone());
    Harness { _dir: dir, engine, fake, secrets, root }
}

fn build(root: &Path, secrets: Arc<dyn SecretStore>, fake: Arc<Fake>) -> Engine {
    let store = Store::open(&root.join("data/manager.db")).unwrap();
    let vault = Vault::open(&root.join("data/vault"), secrets.clone()).unwrap();
    Engine::assemble(store, vault, secrets, vec![fake as Arc<dyn Provider>], root.join("data"))
}

fn sign_in(h: &Harness, user: &str, token: &str) {
    std::fs::write(&h.fake.file, serde_json::json!({"user": user, "token": token}).to_string()).unwrap();
}

fn live(h: &Harness) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(&h.fake.file).unwrap()).unwrap()
}

fn quiet(_: Progress) {}

#[test]
fn save_current_dedupes_and_marks_active() {
    let h = harness(false);
    sign_in(&h, "a", "t1");
    let a = h.engine.save_current(ProviderId::Codex, Some("Personal".into()), None).unwrap();
    let again = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    assert_eq!(a.id, again.id);
    assert_eq!(again.label, "Personal");
    let views = h.engine.account_views(None).unwrap();
    assert_eq!(views.len(), 1);
    assert!(views[0].active);
}

#[test]
fn switch_activates_verifies_and_records() {
    let h = harness(false);
    sign_in(&h, "a", "t1");
    let a = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    sign_in(&h, "b", "t2");
    let b = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    let out = h.engine.switch(&a.id, SwitchOptions::default(), &quiet).unwrap();
    assert_eq!(out.status, "succeeded");
    assert_eq!(live(&h)["user"], "a");
    let hist = h.engine.history(10).unwrap();
    assert_eq!(hist[0].status, "succeeded");
    assert_eq!(hist[0].from_account_id.as_deref(), Some(b.id.as_str()));
    let again = h.engine.switch(&a.id, SwitchOptions::default(), &quiet).unwrap();
    assert_eq!(again.status, "already-active");
}

#[test]
fn outgoing_rotated_tokens_are_preserved() {
    let h = harness(false);
    sign_in(&h, "a", "t1");
    let a = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    sign_in(&h, "b", "t2");
    let b = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    sign_in(&h, "b", "t2-rotated");
    h.engine.switch(&a.id, SwitchOptions::default(), &quiet).unwrap();
    h.engine.switch(&b.id, SwitchOptions::default(), &quiet).unwrap();
    assert_eq!(live(&h)["token"], "t2-rotated");
}

#[test]
fn failed_verification_rolls_back_exactly() {
    let h = harness(false);
    sign_in(&h, "a", "t1");
    let a = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    sign_in(&h, "b", "t2");
    h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    let before = std::fs::read(&h.fake.file).unwrap();
    h.fake.fail_verify.store(true, Ordering::SeqCst);
    let err = h.engine.switch(&a.id, SwitchOptions::default(), &quiet).unwrap_err();
    assert!(err.to_string().contains("previous account has been restored"));
    assert_eq!(std::fs::read(&h.fake.file).unwrap(), before);
    assert_eq!(h.engine.history(1).unwrap()[0].status, "rolled-back");
}

#[test]
fn interrupted_switch_can_be_recovered() {
    let h = harness(false);
    sign_in(&h, "a", "t1");
    let a = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    sign_in(&h, "b", "t2");
    let b = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    h.engine.switch(&a.id, SwitchOptions::default(), &quiet).unwrap();
    let op = h.engine.history(1).unwrap()[0].id.clone();
    h.engine.db().set_switch_status(&op, "applying").unwrap();
    drop(h.engine);
    let engine = build(&h.root, h.secrets.clone(), h.fake.clone());
    let pending = engine.recover_on_launch().unwrap();
    assert_eq!(pending.len(), 1);
    engine.resolve_incomplete(&pending[0].id, RecoveryAction::RestorePrevious).unwrap();
    let now: serde_json::Value = serde_json::from_slice(&std::fs::read(&h.fake.file).unwrap()).unwrap();
    assert_eq!(now["user"], "b");
    assert!(engine.incomplete().unwrap().is_empty());
    let views = engine.account_views(None).unwrap();
    assert!(views.iter().find(|v| v.row.id == b.id).unwrap().active);
}

#[test]
fn concurrent_switch_is_rejected() {
    let h = harness(false);
    sign_in(&h, "a", "t1");
    let a = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    let _g = h.engine.lock(ProviderId::Codex).unwrap();
    assert!(matches!(h.engine.switch(&a.id, SwitchOptions::default(), &quiet), Err(Error::Busy(_))));
}

#[test]
fn usage_only_provider_never_switches() {
    let h = harness(true);
    sign_in(&h, "a", "t1");
    let a = h.engine.save_current(ProviderId::Antigravity, None, None).unwrap();
    sign_in(&h, "b", "t2");
    h.engine.save_current(ProviderId::Antigravity, None, None).unwrap();
    assert_eq!(h.engine.plan_switch(&a.id).unwrap().blocked.as_deref(), Some(altflip_core::ANTIGRAVITY_USAGE_ONLY));
    assert!(matches!(h.engine.switch(&a.id, SwitchOptions::default(), &quiet), Err(Error::Incompatible(_))));
    let o = h.engine.overview(false).unwrap();
    assert!(!o.providers[0].switching_enabled);
}

#[test]
fn drift_is_detected_and_external_switch_adopted() {
    let h = harness(false);
    sign_in(&h, "a", "t1");
    let a = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    sign_in(&h, "b", "t2");
    let b = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    sign_in(&h, "zz", "t3");
    let o = h.engine.overview(true).unwrap();
    assert_eq!(o.providers[0].activation_state.as_deref(), Some("drifted"));
    assert_eq!(o.providers[0].active_account_id.as_deref(), Some(b.id.as_str()));
    sign_in(&h, "a", "t1");
    let o = h.engine.overview(true).unwrap();
    assert_eq!(o.providers[0].active_account_id.as_deref(), Some(a.id.as_str()));
    assert_eq!(o.providers[0].activation_state.as_deref(), Some("external"));
}

#[test]
fn backup_roundtrip_into_fresh_install() {
    let h = harness(false);
    sign_in(&h, "a", "t1");
    h.engine.save_current(ProviderId::Codex, Some("Main".into()), None).unwrap();
    let file = h.root.join("backup.altflip");
    assert_eq!(h.engine.export_backup(&file, "long passphrase").unwrap(), 1);
    let raw = std::fs::read(&file).unwrap();
    assert!(!raw.windows(2).any(|w| w == b"t1"));
    let dir2 = tempfile::tempdir().unwrap();
    let secrets2: Arc<dyn SecretStore> = Arc::new(MemorySecrets::default());
    let e2 = build(dir2.path(), secrets2, h.fake.clone());
    assert!(e2.import_backup(&file, "wrong passphrase").is_err());
    assert_eq!(e2.import_backup(&file, "long passphrase").unwrap(), 1);
    let rows = e2.account_views(None).unwrap();
    assert_eq!(rows[0].row.label, "Main");
}

#[test]
fn remove_account_keeps_provider_signin() {
    let h = harness(false);
    sign_in(&h, "a", "t1");
    let a = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    h.engine.remove_account(&a.id).unwrap();
    assert!(h.engine.account_views(None).unwrap().is_empty());
    assert_eq!(live(&h)["user"], "a");
    assert!(!h.root.join(format!("data/vault/accounts/{}.bin", a.id)).exists());
}

fn reading(kind: &str, at: &str, key: Option<&str>) -> altflip_core::UsageReading {
    altflip_core::UsageReading {
        usage_type: kind.into(),
        label: kind.into(),
        used_percent: Some(40.0),
        used_value: None,
        limit_value: None,
        window_minutes: None,
        reset_at: None,
        source: "test".into(),
        confidence: "provider-reported".into(),
        observed_at: at.into(),
        account_key: key.map(str::to_string),
    }
}

#[test]
fn usage_is_attributed_by_identity_or_to_account_active_at_that_time() {
    let h = harness(false);
    sign_in(&h, "a", "t1");
    let a = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    sign_in(&h, "b", "t2");
    let b = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    *h.fake.readings.lock().unwrap() =
        vec![reading("old", "2020-01-01T00:00:00Z", None), reading("keyed", "2020-01-01T00:00:00Z", Some("u:a"))];
    let rows = h.engine.refresh_usage().unwrap();
    assert!(rows.iter().any(|r| r.account_id == b.id && r.reading.usage_type == "old"));
    assert!(rows.iter().any(|r| r.account_id == a.id && r.reading.usage_type == "keyed"));
    h.engine.switch(&a.id, SwitchOptions::default(), &quiet).unwrap();
    *h.fake.readings.lock().unwrap() =
        vec![reading("before", "2020-01-02T00:00:00Z", None), reading("after", "2999-01-01T00:00:00Z", None)];
    let rows = h.engine.refresh_usage().unwrap();
    assert!(rows.iter().any(|r| r.account_id == b.id && r.reading.usage_type == "before"));
    assert!(rows.iter().any(|r| r.account_id == a.id && r.reading.usage_type == "after"));
}

#[test]
fn unreadable_live_login_never_overwrites_saved_copy() {
    let h = harness(false);
    sign_in(&h, "a", "t1");
    let a = h.engine.save_current(ProviderId::Codex, None, None).unwrap();
    let before = h.engine.load_snapshot(&h.engine.db().account(&a.id).unwrap()).unwrap();
    h.fake.broken.store(true, Ordering::SeqCst);
    sign_in(&h, "a", "corrupted");
    h.engine.overview(true).unwrap();
    let after = h.engine.load_snapshot(&h.engine.db().account(&a.id).unwrap()).unwrap();
    assert!(after.same_primary_data(&before));
    assert!(h.engine.save_current(ProviderId::Codex, None, None).is_err());
}
