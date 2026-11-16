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
        for u in p.usage().iter().take(8) {
            println!(
                "             usage {:<32} used={:>5.1}% reset={:?} for={}",
                u.label,
                u.used_percent.unwrap_or(-1.0),
                u.reset_at,
                u.account_key.as_deref().map(mask).unwrap_or_else(|| "active".into())
            );
        }
    }
}
