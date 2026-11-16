use crate::secrets::SecretStore;
use crate::{fsutil, Error, Result};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zeroize::Zeroizing;

pub const VAULT_SERVICE: &str = "io.coreify.altflip.vault";
pub const LEGACY_VAULT_SERVICE: &str = "dev.altflip.vault";
pub const VAULT_ACCOUNT: &str = "master-key";
const MAGIC: &[u8; 4] = b"AFV1";
const PASS_MAGIC: &[u8; 4] = b"AFP1";
const PBKDF2_ROUNDS: u32 = 600_000;

pub struct Vault {
    key: Zeroizing<[u8; 32]>,
    dir: PathBuf,
}

fn random<const N: usize>() -> Result<[u8; N]> {
    let mut out = [0u8; N];
    SystemRandom::new().fill(&mut out).map_err(|_| Error::Platform("Secure random generator unavailable".into()))?;
    Ok(out)
}

fn seal_with(key: &[u8; 32], aad: &[u8], plain: &[u8]) -> Result<Vec<u8>> {
    let k = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).map_err(|_| Error::Platform("Bad key".into()))?);
    let nonce_bytes = random::<NONCE_LEN>()?;
    let mut buf = plain.to_vec();
    k.seal_in_place_append_tag(Nonce::assume_unique_for_key(nonce_bytes), Aad::from(aad), &mut buf)
        .map_err(|_| Error::Platform("Encryption failed".into()))?;
    let mut out = Vec::with_capacity(NONCE_LEN + buf.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&buf);
    Ok(out)
}

fn open_with(key: &[u8; 32], aad: &[u8], sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if sealed.len() < NONCE_LEN + 16 {
        return Err(Error::Credential("Encrypted data is truncated".into()));
    }
    let k = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).map_err(|_| Error::Platform("Bad key".into()))?);
    let nonce =
        Nonce::try_assume_unique_for_key(&sealed[..NONCE_LEN]).map_err(|_| Error::Credential("Bad nonce".into()))?;
    let mut buf = Zeroizing::new(sealed[NONCE_LEN..].to_vec());
    let len = k
        .open_in_place(nonce, Aad::from(aad), &mut buf)
        .map_err(|_| Error::Credential("Encrypted data could not be decrypted (wrong key or corrupted)".into()))?
        .len();
    buf.truncate(len);
    Ok(buf)
}

impl Vault {
    pub fn open(dir: &Path, secrets: Arc<dyn SecretStore>) -> Result<Self> {
        std::fs::create_dir_all(dir).map_err(|e| Error::io(format!("Cannot create {}", dir.display()), e))?;
        fsutil::restrict_dir(dir);
        if secrets.get(VAULT_SERVICE, VAULT_ACCOUNT)?.is_none() {
            if let Some(old) = secrets.get(LEGACY_VAULT_SERVICE, VAULT_ACCOUNT)? {
                secrets.set(VAULT_SERVICE, VAULT_ACCOUNT, &old)?;
                let _ = secrets.delete(LEGACY_VAULT_SERVICE, VAULT_ACCOUNT);
            }
        }
        let key = match secrets.get(VAULT_SERVICE, VAULT_ACCOUNT)? {
            Some(hexed) => {
                let raw = Zeroizing::new(
                    hex::decode(&hexed).map_err(|_| Error::Credential("The AltFlip vault key is malformed".into()))?,
                );
                let arr: [u8; 32] = raw
                    .as_slice()
                    .try_into()
                    .map_err(|_| Error::Credential("The AltFlip vault key is malformed".into()))?;
                Zeroizing::new(arr)
            }
            None => {
                if std::fs::read_dir(dir).map(|mut d| d.next().is_some()).unwrap_or(false) {
                    return Err(Error::Credential(
                        "The AltFlip vault key is missing from the credential store, so saved accounts cannot be decrypted"
                            .into(),
                    ));
                }
                let k = Zeroizing::new(random::<32>()?);
                let hexed = Zeroizing::new(hex::encode(*k));
                secrets.set(VAULT_SERVICE, VAULT_ACCOUNT, hexed.as_bytes())?;
                k
            }
        };
        Ok(Vault { key, dir: dir.to_path_buf() })
    }

    pub fn with_key(dir: &Path, key: [u8; 32]) -> Self {
        let _ = std::fs::create_dir_all(dir);
        Vault { key: Zeroizing::new(key), dir: dir.to_path_buf() }
    }

    fn path(&self, reference: &str) -> Result<PathBuf> {
        if reference.is_empty()
            || !reference.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/')
        {
            return Err(Error::Invalid("Invalid vault reference".into()));
        }
        if reference.split('/').any(|p| p.is_empty() || p == "..") {
            return Err(Error::Invalid("Invalid vault reference".into()));
        }
        Ok(self.dir.join(format!("{reference}.bin")))
    }

    pub fn seal(&self, reference: &str, plain: &[u8]) -> Result<()> {
        let mut out = MAGIC.to_vec();
        out.extend(seal_with(&self.key, reference.as_bytes(), plain)?);
        fsutil::write_private(&self.path(reference)?, &out)
    }

    pub fn open_ref(&self, reference: &str) -> Result<Zeroizing<Vec<u8>>> {
        let p = self.path(reference)?;
        let data = std::fs::read(&p).map_err(|e| Error::io(format!("Saved credential {reference} is missing"), e))?;
        if !data.starts_with(MAGIC) {
            return Err(Error::Credential(format!("Saved credential {reference} has an unknown format")));
        }
        open_with(&self.key, reference.as_bytes(), &data[4..])
    }

    pub fn exists(&self, reference: &str) -> bool {
        self.path(reference).map(|p| p.is_file()).unwrap_or(false)
    }

    pub fn remove(&self, reference: &str) -> Result<()> {
        fsutil::remove_if_exists(&self.path(reference)?)
    }
}

fn derive(pass: &str, salt: &[u8]) -> Zeroizing<[u8; 32]> {
    let mut key = Zeroizing::new([0u8; 32]);
    ring::pbkdf2::derive(
        ring::pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(PBKDF2_ROUNDS).unwrap(),
        salt,
        pass.as_bytes(),
        key.as_mut(),
    );
    key
}

pub fn seal_with_passphrase(pass: &str, plain: &[u8]) -> Result<Vec<u8>> {
    if pass.chars().count() < 8 {
        return Err(Error::Invalid("Use a passphrase of at least 8 characters".into()));
    }
    let salt = random::<16>()?;
    let key = derive(pass, &salt);
    let mut out = PASS_MAGIC.to_vec();
    out.extend_from_slice(&salt);
    out.extend(seal_with(&key, PASS_MAGIC, plain)?);
    Ok(out)
}

pub fn open_with_passphrase(pass: &str, data: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if !data.starts_with(PASS_MAGIC) || data.len() < 20 {
        return Err(Error::Invalid("This is not an AltFlip backup file".into()));
    }
    let key = derive(pass, &data[4..20]);
    open_with(&key, PASS_MAGIC, &data[20..])
        .map_err(|_| Error::Credential("Wrong passphrase, or the backup file is damaged".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::MemorySecrets;

    #[test]
    fn seals_and_opens_with_bound_reference() {
        let dir = tempfile::tempdir().unwrap();
        let secrets: Arc<dyn SecretStore> = Arc::new(MemorySecrets::default());
        let v = Vault::open(dir.path(), secrets.clone()).unwrap();
        v.seal("accounts/a1", b"secret").unwrap();
        assert_eq!(v.open_ref("accounts/a1").unwrap().as_slice(), b"secret");
        let raw = std::fs::read(dir.path().join("accounts/a1.bin")).unwrap();
        assert!(!raw.windows(6).any(|w| w == b"secret"));
        std::fs::copy(dir.path().join("accounts/a1.bin"), dir.path().join("accounts/a2.bin")).unwrap();
        assert!(v.open_ref("accounts/a2").is_err());
        let again = Vault::open(dir.path(), secrets).unwrap();
        assert_eq!(again.open_ref("accounts/a1").unwrap().as_slice(), b"secret");
        assert!(v.path("../x").is_err());
    }

    #[test]
    fn refuses_new_key_when_data_exists() {
        let dir = tempfile::tempdir().unwrap();
        Vault::open(dir.path(), Arc::new(MemorySecrets::default())).unwrap().seal("a", b"x").unwrap();
        assert!(Vault::open(dir.path(), Arc::new(MemorySecrets::default())).is_err());
    }

    #[test]
    fn migrates_legacy_key_name() {
        let dir = tempfile::tempdir().unwrap();
        let secrets: Arc<dyn SecretStore> = Arc::new(MemorySecrets::default());
        let key = [7u8; 32];
        Vault::with_key(dir.path(), key).seal("a", b"x").unwrap();
        secrets.set(LEGACY_VAULT_SERVICE, VAULT_ACCOUNT, hex::encode(key).as_bytes()).unwrap();
        let v = Vault::open(dir.path(), secrets.clone()).unwrap();
        assert_eq!(v.open_ref("a").unwrap().as_slice(), b"x");
        assert!(secrets.get(LEGACY_VAULT_SERVICE, VAULT_ACCOUNT).unwrap().is_none());
        assert!(secrets.get(VAULT_SERVICE, VAULT_ACCOUNT).unwrap().is_some());
    }

    #[test]
    fn passphrase_roundtrip() {
        let sealed = seal_with_passphrase("correct horse", b"payload").unwrap();
        assert_eq!(open_with_passphrase("correct horse", &sealed).unwrap().as_slice(), b"payload");
        assert!(open_with_passphrase("wrong pass", &sealed).is_err());
        assert!(seal_with_passphrase("short", b"x").is_err());
    }
}
