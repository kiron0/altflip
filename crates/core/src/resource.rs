use crate::secrets::SecretStore;
use crate::{fsutil, Error, Identity, ProviderId, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use zeroize::Zeroize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Locator {
    File { path: PathBuf },
    JsonKey { path: PathBuf, key: String },
    Credential { service: String, account: String },
    StateKey { path: PathBuf, key: String },
}

impl Locator {
    pub fn describe(&self) -> String {
        match self {
            Locator::File { path } => path.display().to_string(),
            Locator::JsonKey { path, key } => format!("{} ({key})", path.display()),
            Locator::Credential { service, .. } => format!("credential store: {service}"),
            Locator::StateKey { path, key } => format!("{} ({key})", path.display()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Primary,
    Secondary,
}

mod b64 {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(b) => s.serialize_some(&base64::engine::general_purpose::STANDARD.encode(b)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
        let o: Option<String> = Option::deserialize(d)?;
        o.map(|s| base64::engine::general_purpose::STANDARD.decode(s).map_err(serde::de::Error::custom)).transpose()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Resource {
    pub locator: Locator,
    pub role: Role,
    #[serde(with = "b64")]
    pub data: Option<Vec<u8>>,
}

impl Drop for Resource {
    fn drop(&mut self) {
        if let Some(d) = self.data.as_mut() {
            d.zeroize();
        }
    }
}

impl std::fmt::Debug for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resource")
            .field("locator", &self.locator)
            .field("role", &self.role)
            .field("bytes", &self.data.as_ref().map(|d| d.len()))
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub version: u32,
    pub provider: ProviderId,
    pub identity: Identity,
    pub captured_at: String,
    pub resources: Vec<Resource>,
}

impl Snapshot {
    pub fn new(provider: ProviderId, identity: Identity, resources: Vec<Resource>) -> Self {
        Snapshot { version: 1, provider, identity, captured_at: crate::now_rfc3339(), resources }
    }

    pub fn primary(&self) -> impl Iterator<Item = &Resource> {
        self.resources.iter().filter(|r| r.role == Role::Primary)
    }

    pub fn data_for(&self, locator: &Locator) -> Option<&[u8]> {
        self.resources.iter().find(|r| &r.locator == locator).and_then(|r| r.data.as_deref())
    }

    pub fn to_bytes(&self) -> Result<zeroize::Zeroizing<Vec<u8>>> {
        Ok(zeroize::Zeroizing::new(serde_json::to_vec(self)?))
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(b)?)
    }

    pub fn same_primary_data(&self, other: &Snapshot) -> bool {
        self.primary().all(|r| other.data_for(&r.locator) == r.data.as_deref())
    }
}

#[derive(Clone)]
pub struct ResourceIo {
    pub secrets: Arc<dyn SecretStore>,
}

impl ResourceIo {
    pub fn new(secrets: Arc<dyn SecretStore>) -> Self {
        ResourceIo { secrets }
    }

    pub fn read(&self, locator: &Locator) -> Result<Option<Vec<u8>>> {
        match locator {
            Locator::File { path } => fsutil::read_optional(path),
            Locator::JsonKey { path, key } => {
                let Some(raw) = fsutil::read_optional(path)? else { return Ok(None) };
                let v: serde_json::Value = serde_json::from_slice(&raw)
                    .map_err(|e| Error::Invalid(format!("{} is not valid JSON: {e}", path.display())))?;
                match v.get(key) {
                    None | Some(serde_json::Value::Null) => Ok(None),
                    Some(x) => Ok(Some(serde_json::to_vec(x)?)),
                }
            }
            Locator::Credential { service, account } => self.secrets.get(service, account),
            Locator::StateKey { path, key } => crate::statedb::read(path, key),
        }
    }

    pub fn capture(&self, locator: Locator, role: Role) -> Result<Resource> {
        let data = self.read(&locator)?;
        Ok(Resource { locator, role, data })
    }

    pub fn write(&self, locator: &Locator, data: &[u8]) -> Result<()> {
        match locator {
            Locator::File { path } => fsutil::atomic_write(path, data),
            Locator::JsonKey { path, key } => {
                let value: serde_json::Value = serde_json::from_slice(data)?;
                let mut doc = match fsutil::read_optional(path)? {
                    Some(raw) => serde_json::from_slice::<serde_json::Value>(&raw)
                        .map_err(|e| Error::Invalid(format!("{} is not valid JSON: {e}", path.display())))?,
                    None => serde_json::Value::Object(Default::default()),
                };
                let obj = doc
                    .as_object_mut()
                    .ok_or_else(|| Error::Invalid(format!("{} is not a JSON object", path.display())))?;
                obj.insert(key.clone(), value);
                let mut out = serde_json::to_vec_pretty(&doc)?;
                out.push(b'\n');
                fsutil::atomic_write(path, &out)
            }
            Locator::Credential { service, account } => self.secrets.set(service, account, data),
            Locator::StateKey { path, key } => crate::statedb::write(path, key, data),
        }
    }

    pub fn remove(&self, locator: &Locator) -> Result<()> {
        match locator {
            Locator::File { path } => fsutil::remove_if_exists(path),
            Locator::JsonKey { path, key } => {
                let Some(raw) = fsutil::read_optional(path)? else { return Ok(()) };
                let mut doc: serde_json::Value = serde_json::from_slice(&raw)?;
                if let Some(obj) = doc.as_object_mut() {
                    if obj.remove(key).is_some() {
                        let mut out = serde_json::to_vec_pretty(&doc)?;
                        out.push(b'\n');
                        fsutil::atomic_write(path, &out)?;
                    }
                }
                Ok(())
            }
            Locator::Credential { service, account } => self.secrets.delete(service, account),
            Locator::StateKey { path, key } => crate::statedb::delete(path, key),
        }
    }

    pub fn apply(&self, snap: &Snapshot) -> Result<()> {
        for r in &snap.resources {
            match (&r.data, r.role) {
                (Some(d), _) => self.write(&r.locator, d)?,
                (None, Role::Primary) => {
                    return Err(Error::Invalid(format!(
                        "The saved account is missing required data for {}",
                        r.locator.describe()
                    )))
                }
                (None, Role::Secondary) => {}
            }
        }
        Ok(())
    }

    pub fn restore(&self, snap: &Snapshot) -> Result<()> {
        let mut first_err = None;
        for r in &snap.resources {
            let res = match &r.data {
                Some(d) => self.write(&r.locator, d),
                None => self.remove(&r.locator),
            };
            if let Err(e) = res {
                first_err.get_or_insert(e);
            }
        }
        first_err.map_or(Ok(()), Err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::MemorySecrets;

    #[test]
    fn json_key_write_preserves_other_keys_and_order() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("c.json");
        std::fs::write(&p, br#"{"z":1,"oauthAccount":{"e":"a"},"a":[1,2]}"#).unwrap();
        let io = ResourceIo::new(Arc::new(MemorySecrets::default()));
        let loc = Locator::JsonKey { path: p.clone(), key: "oauthAccount".into() };
        assert_eq!(io.read(&loc).unwrap().unwrap(), br#"{"e":"a"}"#);
        io.write(&loc, br#"{"e":"b"}"#).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
        assert_eq!(v["oauthAccount"]["e"], "b");
        assert_eq!(v["a"][1], 2);
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, vec!["z", "oauthAccount", "a"]);
    }

    #[test]
    fn apply_requires_primary_and_restore_removes_absent() {
        let dir = tempfile::tempdir().unwrap();
        let io = ResourceIo::new(Arc::new(MemorySecrets::default()));
        let f = Locator::File { path: dir.path().join("f") };
        let c = Locator::Credential { service: "s".into(), account: "a".into() };
        let snap = Snapshot::new(
            ProviderId::Codex,
            Identity::default(),
            vec![
                Resource { locator: f.clone(), role: Role::Primary, data: None },
                Resource { locator: c.clone(), role: Role::Secondary, data: Some(b"x".to_vec()) },
            ],
        );
        assert!(io.apply(&snap).is_err());
        std::fs::write(dir.path().join("f"), b"old").unwrap();
        io.restore(&snap).unwrap();
        assert!(!dir.path().join("f").exists());
        assert_eq!(io.read(&c).unwrap().unwrap(), b"x");
    }

    #[test]
    fn snapshot_roundtrip_and_debug_hides_data() {
        let snap = Snapshot::new(
            ProviderId::Claude,
            Identity { key: "k".into(), ..Default::default() },
            vec![Resource {
                locator: Locator::File { path: "/x".into() },
                role: Role::Primary,
                data: Some(b"tok".to_vec()),
            }],
        );
        let back = Snapshot::from_bytes(&snap.to_bytes().unwrap()).unwrap();
        assert!(back.same_primary_data(&snap));
        assert!(!format!("{back:?}").contains("tok"));
    }
}
