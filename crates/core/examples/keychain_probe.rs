use altflip_core::secrets::{SecretStore, SystemSecrets};

fn main() {
    let s = SystemSecrets;
    let svc = "AltFlip Probe-roundtrip";
    for n in [100usize, 2500, 5000] {
        let v: Vec<u8> = (0..n).map(|i| b'a' + (i % 26) as u8).collect();
        s.set(svc, "probe", &v).unwrap();
        assert_eq!(s.get(svc, "probe").unwrap().unwrap(), v, "len {n}");
        println!("ok text {n}");
    }
    let hex_text = b"84b0aa11ff2299cc84b0aa11ff2299cc84b0aa11ff2299cc84b0aa11ff2299cc".to_vec();
    s.set(svc, "probe", &hex_text).unwrap();
    assert_eq!(s.get(svc, "probe").unwrap().unwrap(), hex_text);
    println!("ok hex text");
    let real = s.get(altflip_core::vault::VAULT_SERVICE, altflip_core::vault::VAULT_ACCOUNT).unwrap();
    println!("vault key decodes: {}", real.map(|k| hex::decode(&k).map(|d| d.len() == 32).unwrap_or(false)).unwrap_or(false));
    let bin: Vec<u8> = vec![7, 34, 109, 0x7b, 10, 200];
    s.set(svc, "probe", &bin).unwrap();
    assert_eq!(s.get(svc, "probe").unwrap().unwrap(), bin);
    println!("ok binary");
    s.delete(svc, "probe").unwrap();
    assert!(s.get(svc, "probe").unwrap().is_none());
    println!("cleaned");
}
