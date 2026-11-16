pub mod antigravity;
pub mod claude;
pub mod claude_status;
pub mod claude_usage;
pub mod codex;
pub mod opencode;
pub mod pb;

use altflip_core::process;
use altflip_core::resource::{Locator, ResourceIo, Role, Snapshot};
use altflip_core::{
    AuthHealth, Capabilities, Compatibility, Error, Identity, ProcessInfo, ProviderId, ProviderInfo, Result,
    SwitchMode, UsageReading,
};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct Ctx {
    pub home: PathBuf,
    pub io: ResourceIo,
    pub temp_dir: PathBuf,
}

pub type OutputSink<'a> = &'a (dyn Fn(&str) + Send + Sync);

pub struct Detection {
    pub installed: bool,
    pub version: Option<String>,
    pub binary_path: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
    pub compatibility: Compatibility,
    pub note: Option<String>,
}

pub trait Provider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn ctx(&self) -> &Ctx;
    fn experimental(&self) -> bool {
        false
    }
    fn capabilities(&self) -> Capabilities;
    fn detection(&self) -> Detection;
    fn locators(&self) -> Vec<(Locator, Role)>;
    fn identify(&self, snap: &Snapshot) -> Result<Option<Identity>>;
    fn health(&self, snap: &Snapshot) -> AuthHealth;
    fn processes(&self) -> Vec<ProcessInfo>;

    fn mode(&self, _procs: &[ProcessInfo]) -> SwitchMode {
        SwitchMode::NewSessions
    }

    fn restart_targets(&self, _procs: &[ProcessInfo]) -> Vec<ProcessInfo> {
        Vec::new()
    }

    fn capture(&self) -> Result<Option<Snapshot>> {
        let mut resources = Vec::new();
        for (loc, role) in self.locators() {
            resources.push(self.ctx().io.capture(loc, role)?);
        }
        let mut snap = Snapshot::new(self.id(), Identity::default(), resources);
        self.normalize(&mut snap);
        match self.identify(&snap)? {
            Some(id) => {
                snap.identity = id;
                Ok(Some(snap))
            }
            None => Ok(None),
        }
    }

    fn normalize(&self, _snap: &mut Snapshot) {}

    fn raw_capture(&self) -> Result<Snapshot> {
        let mut resources = Vec::new();
        for (loc, role) in self.locators() {
            resources.push(self.ctx().io.capture(loc, role)?);
        }
        Ok(Snapshot::new(self.id(), Identity::default(), resources))
    }

    fn import(&self) -> Result<Vec<Snapshot>> {
        Err(Error::Unsupported(format!("{} cannot import stored sign-ins", self.id())))
    }

    fn apply(&self, snap: &Snapshot) -> Result<()> {
        if snap.provider != self.id() {
            return Err(Error::Invalid("Saved account belongs to a different provider".into()));
        }
        self.ctx().io.apply(snap)
    }

    fn restore(&self, snap: &Snapshot) -> Result<()> {
        self.ctx().io.restore(snap)
    }

    fn verify(&self, expected: &Snapshot) -> Result<String> {
        let live = self.capture()?.ok_or_else(|| {
            Error::Verification(format!("{} reports no signed-in account after switching", self.id()))
        })?;
        if live.identity.key != expected.identity.key {
            return Err(Error::Verification(format!(
                "{} is signed in as {} instead of {}",
                self.id(),
                live.identity.short(),
                expected.identity.short()
            )));
        }
        if !expected.same_primary_data(&live) {
            return Err(Error::Verification("Stored credentials do not match the selected account".into()));
        }
        Ok(format!("Credential files and stores now hold {}", expected.identity.short()))
    }

    fn login(&self, _cancel: &AtomicBool, _out: OutputSink) -> Result<Snapshot> {
        Err(Error::Unsupported(format!(
            "{} cannot sign in a new account in isolation. Sign in with {} itself, then use Save current account.",
            self.id(),
            self.id()
        )))
    }

    fn usage(&self) -> Vec<UsageReading> {
        Vec::new()
    }

    fn info(&self) -> ProviderInfo {
        let d = self.detection();
        let processes = if d.installed { self.processes() } else { Vec::new() };
        let switch_mode = match d.compatibility {
            Compatibility::Unsupported | Compatibility::NotInstalled => SwitchMode::Unsupported,
            _ => self.mode(&processes),
        };
        ProviderInfo {
            id: self.id(),
            name: self.id().name().into(),
            installed: d.installed,
            version: d.version,
            binary_path: d.binary_path,
            config_path: d.config_path,
            compatibility: d.compatibility,
            compatibility_note: d.note,
            experimental: self.experimental(),
            capabilities: self.capabilities(),
            processes,
            switch_mode,
        }
    }
}

pub fn registry(ctx: Ctx) -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(claude::Claude::new(ctx.clone())),
        Arc::new(codex::Codex::new(ctx.clone())),
        Arc::new(antigravity::Antigravity::new(ctx.clone())),
        Arc::new(opencode::OpenCode::new(ctx)),
    ]
}

type VersionCache = std::collections::HashMap<std::path::PathBuf, (Option<std::time::SystemTime>, Option<String>)>;
static VERSIONS: std::sync::Mutex<Option<VersionCache>> = std::sync::Mutex::new(None);

pub(crate) fn version_of(bin: &std::path::Path) -> Option<String> {
    let stamp = std::fs::metadata(bin).and_then(|m| m.modified()).ok();
    if let Some((s, v)) = VERSIONS.lock().ok().and_then(|m| m.as_ref().and_then(|m| m.get(bin).cloned())) {
        if s == stamp {
            return v;
        }
    }
    let v = process::run_capture(bin, &["--version"], &[], Duration::from_secs(8))
        .ok()
        .and_then(|out| out.lines().next().map(|l| l.trim().to_string()))
        .filter(|s| !s.is_empty());
    if let Ok(mut m) = VERSIONS.lock() {
        m.get_or_insert_with(Default::default).insert(bin.to_path_buf(), (stamp, v.clone()));
    }
    v
}

pub(crate) fn parse_semver(s: &str) -> Option<(u64, u64, u64)> {
    let token = s
        .split(|c: char| c.is_whitespace() || c == 'v')
        .find(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit()))?;
    let mut it = token.split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()).map(|p| p.parse::<u64>().ok());
    Some((it.next()??, it.next().flatten().unwrap_or(0), it.next().flatten().unwrap_or(0)))
}

pub(crate) fn json(bytes: Option<&[u8]>) -> Option<serde_json::Value> {
    bytes.and_then(|b| serde_json::from_slice(b).ok())
}

pub(crate) fn epoch_to_rfc3339(secs: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(secs, 0).map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

fn drain(rx: &std::sync::mpsc::Receiver<String>, out: OutputSink) {
    while let Ok(line) = rx.try_recv() {
        let clean = strip_ansi(&line);
        if !clean.trim().is_empty() {
            out(&clean);
        }
    }
}

pub(crate) fn run_login(
    bin: &std::path::Path,
    args: &[&str],
    envs: &[(&str, &std::path::Path)],
    cancel: &AtomicBool,
    out: OutputSink,
    timeout: Duration,
) -> Result<()> {
    use std::io::{BufRead, BufReader, Read};
    use std::process::{Command, Stdio};
    use std::sync::atomic::Ordering;
    let mut cmd = Command::new(bin);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().map_err(|e| Error::Platform(format!("Cannot start {}: {e}", bin.display())))?;
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let streams: Vec<Box<dyn Read + Send>> = [
        child.stdout.take().map(|s| Box::new(s) as Box<dyn Read + Send>),
        child.stderr.take().map(|s| Box::new(s) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    .collect();
    for stream in streams {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(|l| l.ok()) {
                let _ = tx.send(line);
            }
        });
    }
    drop(tx);
    let start = std::time::Instant::now();
    loop {
        drain(&rx, out);
        if cancel.load(Ordering::SeqCst) || start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            let msg = if cancel.load(Ordering::SeqCst) { "Sign-in was cancelled" } else { "Sign-in timed out" };
            return Err(Error::Cancelled(msg.into()));
        }
        match child.try_wait() {
            Ok(Some(st)) => {
                std::thread::sleep(Duration::from_millis(100));
                drain(&rx, out);
                return if st.success() {
                    Ok(())
                } else {
                    Err(Error::Credential(format!("Sign-in exited with code {}", st.code().unwrap_or(-1))))
                };
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(150)),
            Err(e) => return Err(Error::Platform(e.to_string())),
        }
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for n in chars.by_ref() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

pub(crate) fn scratch_dir(ctx: &Ctx, prefix: &str) -> Result<PathBuf> {
    let dir = ctx.temp_dir.join(format!("{prefix}-{}", altflip_core::new_id()));
    std::fs::create_dir_all(&dir).map_err(|e| Error::io("Cannot create a temporary sign-in folder", e))?;
    altflip_core::fsutil::restrict_dir(&dir);
    Ok(dir.canonicalize().unwrap_or(dir))
}

#[cfg(test)]
pub(crate) mod testutil {
    use super::*;
    use altflip_core::secrets::MemorySecrets;

    pub fn ctx() -> (tempfile::TempDir, Ctx) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Ctx {
            home: dir.path().to_path_buf(),
            io: ResourceIo::new(Arc::new(MemorySecrets::default())),
            temp_dir: dir.path().join("tmp"),
        };
        (dir, ctx)
    }

    pub fn jwt(claims: serde_json::Value) -> String {
        use base64::Engine;
        let b = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims.to_string());
        format!("h.{b}.s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_and_ansi() {
        assert_eq!(parse_semver("2.1.312 (Claude Code)"), Some((2, 1, 312)));
        assert_eq!(parse_semver("codex-cli 0.164.0"), Some((0, 164, 0)));
        assert_eq!(strip_ansi("\u{1b}[32mok\u{1b}[0m"), "ok");
    }
}
