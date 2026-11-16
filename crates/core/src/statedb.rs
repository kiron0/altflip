use crate::{Error, Result};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use std::path::Path;
use std::time::Duration;

fn open(path: &Path, write: bool) -> Result<Connection> {
    let flags = if write {
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX
    } else {
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX
    };
    let conn = Connection::open_with_flags(path, flags)
        .map_err(|e| Error::Platform(format!("Cannot open {}: {e}", path.display())))?;
    conn.busy_timeout(Duration::from_secs(5)).map_err(|e| Error::Platform(e.to_string()))?;
    Ok(conn)
}

fn value_bytes(v: rusqlite::types::Value) -> Option<Vec<u8>> {
    match v {
        rusqlite::types::Value::Text(s) => Some(s.into_bytes()),
        rusqlite::types::Value::Blob(b) => Some(b),
        _ => None,
    }
}

pub fn read(path: &Path, key: &str) -> Result<Option<Vec<u8>>> {
    if !path.is_file() {
        return Ok(None);
    }
    let conn = open(path, false)?;
    let v: Option<rusqlite::types::Value> = conn
        .query_row("SELECT value FROM ItemTable WHERE key = ?1", params![key], |r| r.get(0))
        .optional()
        .map_err(|e| Error::Platform(format!("Cannot read {}: {e}", path.display())))?;
    Ok(v.and_then(value_bytes))
}

pub fn write(path: &Path, key: &str, data: &[u8]) -> Result<()> {
    if !path.is_file() {
        return Err(Error::NotFound(format!("{} does not exist", path.display())));
    }
    let conn = open(path, true)?;
    let res = match std::str::from_utf8(data) {
        Ok(s) => conn.execute("INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)", params![key, s]),
        Err(_) => conn.execute("INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)", params![key, data]),
    };
    res.map_err(|e| Error::Platform(format!("Cannot write {}: {e}", path.display())))?;
    match read(path, key)? {
        Some(v) if v == data => Ok(()),
        _ => Err(Error::Verification(format!("{key} did not read back as written"))),
    }
}

pub fn delete(path: &Path, key: &str) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    let conn = open(path, true)?;
    conn.execute("DELETE FROM ItemTable WHERE key = ?1", params![key])
        .map_err(|e| Error::Platform(format!("Cannot write {}: {e}", path.display())))?;
    Ok(())
}

#[cfg(test)]
pub fn create_for_test(path: &Path) {
    let c = Connection::open(path).unwrap();
    c.execute_batch("CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB)").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_keeps_other_keys() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("state.vscdb");
        create_for_test(&p);
        write(&p, "other", b"keep").unwrap();
        assert_eq!(read(&p, "k").unwrap(), None);
        write(&p, "k", b"v1").unwrap();
        write(&p, "k", b"v2").unwrap();
        assert_eq!(read(&p, "k").unwrap().unwrap(), b"v2");
        delete(&p, "k").unwrap();
        assert_eq!(read(&p, "k").unwrap(), None);
        assert_eq!(read(&p, "other").unwrap().unwrap(), b"keep");
        assert_eq!(read(&dir.path().join("missing"), "k").unwrap(), None);
    }
}
