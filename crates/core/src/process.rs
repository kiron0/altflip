use crate::{Error, ProcessInfo, ProcessKind, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

#[derive(Clone)]
pub struct ProcView {
    pub pid: u32,
    pub name: String,
    pub exe: Option<PathBuf>,
    pub cmd: Vec<String>,
}

static SNAPSHOT: std::sync::Mutex<Option<(Instant, Vec<ProcView>)>> = std::sync::Mutex::new(None);

pub fn snapshot() -> Vec<ProcView> {
    if let Ok(cache) = SNAPSHOT.lock() {
        if let Some((at, procs)) = cache.as_ref() {
            if at.elapsed() < Duration::from_millis(1500) {
                return procs.clone();
            }
        }
    }
    let procs = fresh_snapshot();
    if let Ok(mut cache) = SNAPSHOT.lock() {
        *cache = Some((Instant::now(), procs.clone()));
    }
    procs
}

pub fn invalidate() {
    if let Ok(mut cache) = SNAPSHOT.lock() {
        *cache = None;
    }
}

fn fresh_snapshot() -> Vec<ProcView> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet).with_cmd(UpdateKind::OnlyIfNotSet),
    );
    let me = std::process::id();
    sys.processes()
        .iter()
        .filter(|(pid, p)| pid.as_u32() != me && p.thread_kind().is_none())
        .map(|(pid, p)| ProcView {
            pid: pid.as_u32(),
            name: p.name().to_string_lossy().into_owned(),
            exe: p.exe().map(Path::to_path_buf),
            cmd: p.cmd().iter().map(|s| s.to_string_lossy().into_owned()).collect(),
        })
        .collect()
}

pub fn find(classify: impl Fn(&ProcView) -> Option<ProcessKind>) -> Vec<ProcessInfo> {
    snapshot()
        .into_iter()
        .filter_map(|p| {
            classify(&p).map(|kind| ProcessInfo { pid: p.pid, name: p.name.clone(), path: p.exe.clone(), kind })
        })
        .collect()
}

pub fn is_running(pid: u32) -> bool {
    let mut sys = System::new();
    let pid = sysinfo::Pid::from_u32(pid);
    sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing());
    sys.process(pid).is_some()
}

pub fn app_bundle(path: &Path) -> Option<PathBuf> {
    path.ancestors().find(|a| a.extension().is_some_and(|e| e == "app")).map(Path::to_path_buf)
}

pub fn request_quit(target: &ProcessInfo) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        if let Some(bundle) = target.path.as_deref().and_then(app_bundle) {
            let script = format!("tell application \"{}\" to quit", bundle.display().to_string().replace('"', ""));
            let out = std::process::Command::new("/usr/bin/osascript")
                .args(["-e", &script])
                .output()
                .map_err(|e| Error::Platform(format!("Cannot ask {} to quit: {e}", target.name)))?;
            if !out.status.success() {
                return Err(Error::Platform(format!(
                    "{} did not accept the quit request: {}",
                    target.name,
                    String::from_utf8_lossy(&out.stderr).trim()
                )));
            }
            return Ok(());
        }
    }
    #[cfg(unix)]
    {
        let rc = unsafe { libc::kill(target.pid as i32, libc::SIGTERM) };
        if rc != 0 {
            return Err(Error::Platform(format!("Cannot ask {} to quit", target.name)));
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        let out = std::process::Command::new("taskkill")
            .args(["/PID", &target.pid.to_string()])
            .output()
            .map_err(|e| Error::Platform(format!("Cannot ask {} to close: {e}", target.name)))?;
        if !out.status.success() {
            return Err(Error::Platform(format!("{} did not accept the close request", target.name)));
        }
        Ok(())
    }
}

pub fn wait_exit(pids: &[u32], timeout: Duration) -> bool {
    let start = Instant::now();
    loop {
        if pids.iter().all(|p| !is_running(*p)) {
            invalidate();
            return true;
        }
        if start.elapsed() > timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

pub fn launch(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let target = app_bundle(path).unwrap_or_else(|| path.to_path_buf());
        let st = std::process::Command::new("/usr/bin/open")
            .arg("-a")
            .arg(&target)
            .status()
            .map_err(|e| Error::Platform(format!("Cannot reopen {}: {e}", target.display())))?;
        if !st.success() {
            return Err(Error::Platform(format!("Cannot reopen {}", target.display())));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::process::Command::new(path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|e| Error::Platform(format!("Cannot reopen {}: {e}", path.display())))
    }
}

pub fn find_binary(name: &str, extra: &[PathBuf]) -> Option<PathBuf> {
    let exe = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    let mut dirs: Vec<PathBuf> =
        std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    if let Some(h) = dirs::home_dir() {
        dirs.push(h.join(".local/bin"));
        dirs.push(h.join(".claude/local"));
        dirs.push(h.join(".npm-global/bin"));
        dirs.push(h.join(".bun/bin"));
        dirs.push(h.join(".volta/bin"));
        dirs.push(h.join(".cargo/bin"));
        if cfg!(windows) {
            dirs.push(h.join("AppData/Roaming/npm"));
        }
    }
    dirs.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"].map(PathBuf::from));
    dirs.extend(extra.iter().cloned());
    dirs.into_iter().map(|d| d.join(&exe)).find(|p| p.is_file()).or_else(|| {
        if cfg!(windows) {
            let cmd = format!("{name}.cmd");
            std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join(&cmd)).find(|p| p.is_file()))
        } else {
            None
        }
    })
}

pub fn run_capture(bin: &Path, args: &[&str], envs: &[(&str, &Path)], timeout: Duration) -> Result<String> {
    use std::io::Read;
    let mut cmd = std::process::Command::new(bin);
    cmd.args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().map_err(|e| Error::Platform(format!("Cannot run {}: {e}", bin.display())))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                return Err(Error::Platform(format!("{} did not respond in time", bin.display())));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(Error::Platform(e.to_string())),
        }
    }
    let mut s = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_string(&mut s);
    }
    Ok(s)
}
