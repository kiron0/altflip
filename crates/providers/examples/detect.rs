use altflip_core::resource::ResourceIo;
use altflip_core::secrets::SystemSecrets;
use altflip_providers::{registry, Ctx};
use std::sync::Arc;

fn mask(s: &str) -> String {
    match s.split_once('@') {
        Some((u, d)) => format!("{}***@{}", &u[..1.min(u.len())], d),
        None => "***".into(),
    }
}

fn main() {
    let ctx = Ctx {
        home: altflip_core::home_dir().unwrap(),
        io: ResourceIo::new(Arc::new(SystemSecrets)),
        temp_dir: std::env::temp_dir().join("altflip-detect"),
    };
    for p in registry(ctx) {
        let info = p.info();
        println!(
            "{:<12} installed={} version={:?} compat={:?} mode={:?} procs={}",
            info.id.as_str(),
            info.installed,
            info.version,
            info.compatibility,
            info.switch_mode,
            info.processes.len()
        );
        if info.id == altflip_core::ProviderId::Codex {
            if let Ok(Some(snap)) = p.capture() {
                match altflip_providers::codex::live_usage(&snap) {
                    Ok(r) => r.iter().for_each(|u| {
                        println!(
                            "             LIVE {:<20} used={:>5.1}% reset={:?}",
                            u.label,
                            u.used_percent.unwrap_or(-1.0),
                            u.reset_at
                        )
                    }),
                    Err(e) => println!("             LIVE error: {e}"),
                }
            }
        }
        if info.id == altflip_core::ProviderId::Claude {
            if let Ok(Some(snap)) = p.capture() {
                for r in &snap.resources {
                    println!(
                        "             res {:?} role={:?} bytes={:?} json_ok={}",
                        r.locator.describe().chars().take(40).collect::<String>(),
                        r.role,
                        r.data.as_ref().map(|d| d.len()),
                        r.data.as_deref().and_then(|d| serde_json::from_slice::<serde_json::Value>(d).ok()).is_some()
                    );
                }
                if altflip_providers::claude::needs_renewal(&snap) {
                    println!("             LIVE skipped: Claude login needs renewal (the app renews it)");
                } else {
                    match altflip_providers::claude::exact_usage(&snap) {
                        Ok(r) => r.iter().for_each(|u| {
                            println!(
                                "             LIVE {:<20} used={:>5.1}% reset={:?}",
                                u.label,
                                u.used_percent.unwrap_or(-1.0),
                                u.reset_at
                            )
                        }),
                        Err(e) => println!("             LIVE error: {e}"),
                    }
                }
            }
        }
        match p.capture() {
            Ok(Some(s)) => println!(
                "             identity email={} plan={:?} method={:?} health={:?} resources={}",
                s.identity.email.as_deref().map(mask).unwrap_or_default(),
                s.identity.plan,
                s.identity.auth_method,
                p.health(&s),
                s.resources.iter().filter(|r| r.data.is_some()).count()
            ),
            Ok(None) => println!("             not signed in"),
            Err(e) => println!("             error: {e}"),
        }
        for u in p.usage().iter().take(10) {
            println!(
                "             usage {:<32} used={:>5.1}% reset={:?} for={}",
                u.label,
                u.used_percent.unwrap_or(-1.0),
                u.reset_at,
                u.account_key
                    .as_deref()
                    .map(|k| k.chars().take(14).collect::<String>() + "…")
                    .unwrap_or_else(|| "by time".into())
            );
        }
    }
}
