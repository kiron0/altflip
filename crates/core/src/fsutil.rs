use crate::{Error, Result};
use std::io::Write;
use std::path::Path;

pub fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            let target =
                std::fs::metadata(path).map_err(|e| Error::io(format!("Cannot read {}", path.display()), e))?;
            if !target.is_file() {
                return Err(Error::Invalid(format!("{} is not a regular file", path.display())));
            }
        }
        Ok(meta) if !meta.is_file() => {
            return Err(Error::Invalid(format!("{} is not a regular file", path.display())));
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::io(format!("Cannot inspect {}", path.display()), e)),
    }
    std::fs::read(path).map(Some).map_err(|e| Error::io(format!("Cannot read {}", path.display()), e))
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err(Error::Invalid(format!("Refusing to replace {}: not a regular file", path.display())));
        }
    }
    let dir = path.parent().ok_or_else(|| Error::Invalid(format!("{} has no parent directory", path.display())))?;
    std::fs::create_dir_all(dir).map_err(|e| Error::io(format!("Cannot create {}", dir.display()), e))?;
    let mut tmp = tempfile::Builder::new()
        .prefix(".altflip-")
        .tempfile_in(dir)
        .map_err(|e| Error::io(format!("Cannot write in {}", dir.display()), e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path).map(|m| m.permissions().mode() & 0o777).unwrap_or(0o600);
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(mode))
            .map_err(|e| Error::io("Cannot set file permissions", e))?;
    }
    tmp.write_all(bytes).map_err(|e| Error::io(format!("Cannot write {}", path.display()), e))?;
    tmp.as_file().sync_all().map_err(|e| Error::io(format!("Cannot flush {}", path.display()), e))?;
    tmp.persist(path).map_err(|e| Error::io(format!("Cannot replace {}", path.display()), e.error))?;
    #[cfg(unix)]
    if let Ok(d) = std::fs::File::open(dir) {
        let _ = d.sync_all();
    }
    let written = std::fs::read(path).map_err(|e| Error::io(format!("Cannot read back {}", path.display()), e))?;
    if written != bytes {
        return Err(Error::Verification(format!("{} did not contain the expected data after writing", path.display())));
    }
    Ok(())
}

pub fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| Error::io(format!("Cannot create {}", dir.display()), e))?;
        restrict_dir(dir);
    }
    if !path.exists() {
        std::fs::write(path, []).map_err(|e| Error::io(format!("Cannot create {}", path.display()), e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
    }
    atomic_write(path, bytes)
}

pub fn restrict_dir(dir: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    #[cfg(not(unix))]
    let _ = dir;
}

pub fn remove_if_exists(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::io(format!("Cannot remove {}", path.display()), e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_and_verifies() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.json");
        atomic_write(&p, b"one").unwrap();
        atomic_write(&p, b"two").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"two");
        assert_eq!(read_optional(&dir.path().join("missing")).unwrap(), None);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_preserves_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a");
        std::fs::write(&p, b"x").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o640)).unwrap();
        atomic_write(&p, b"y").unwrap();
        assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o640);
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlink_targets() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        std::fs::write(&real, b"x").unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(atomic_write(&link, b"y").is_err());
        assert_eq!(std::fs::read(&real).unwrap(), b"x");
    }
}
