use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::client::PonyMailClient;
use crate::error::{Error, Result};
use crate::models::{ListAddress, Session};

const SESSION_FILE_ENV: &str = "ASFML_SESSION_FILE";

pub fn store_session(session: &Session) -> Result<()> {
    store_file_session(session)
}

pub fn load_session() -> Result<Session> {
    load_file_session()
}

pub fn clear_session() -> Result<()> {
    clear_file_session()
}

pub fn validate_session(client: &PonyMailClient, list: Option<&ListAddress>) -> Result<String> {
    let prefs = client.preferences()?;
    let login = prefs
        .login
        .credentials
        .as_ref()
        .ok_or(Error::InvalidSession)?;
    if let Some(list) = list {
        if !prefs.has_list_access(list) {
            return Err(Error::NoListAccess(list.to_string()));
        }
    }

    let name = login
        .fullname
        .as_deref()
        .or(login.name.as_deref())
        .unwrap_or(&login.uid);
    Ok(match login.email {
        Some(ref email) if !email.is_empty() => format!("{name} <{email}>"),
        _ => name.to_string(),
    })
}

fn store_file_session(session: &Session) -> Result<()> {
    let path = session_file_path()?;
    store_file_session_at(&path, session)
}

fn load_file_session() -> Result<Session> {
    let path = session_file_path()?;
    load_file_session_at(&path)
}

fn clear_file_session() -> Result<()> {
    let path = session_file_path()?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn session_file_path() -> Result<PathBuf> {
    if let Some(path) = env::var_os(SESSION_FILE_ENV).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    Ok(platform_config_dir()?.join("asfml").join("session.json"))
}

#[cfg(target_os = "windows")]
fn platform_config_dir() -> Result<PathBuf> {
    if let Some(appdata) = env::var_os("APPDATA") {
        return Ok(PathBuf::from(appdata));
    }
    if let Some(home) = env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(home).join("AppData").join("Roaming"));
    }
    Err(Error::ConfigDirUnavailable)
}

#[cfg(target_os = "macos")]
fn platform_config_dir() -> Result<PathBuf> {
    env::var_os("HOME")
        .map(|home| {
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
        })
        .ok_or(Error::ConfigDirUnavailable)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_config_dir() -> Result<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or(Error::ConfigDirUnavailable)
}

#[cfg(not(any(unix, target_os = "windows")))]
fn platform_config_dir() -> Result<PathBuf> {
    Err(Error::ConfigDirUnavailable)
}

fn store_file_session_at(path: &Path, session: &Session) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let stored = StoredSession {
        ponymail: session.ponymail.clone(),
    };
    let data = serde_json::to_vec_pretty(&stored)?;
    let temp = path.with_file_name(format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("session.json")
    ));

    write_private_file(&temp, &data)?;
    fs::rename(temp, path)?;
    set_private_permissions(path)?;
    Ok(())
}

fn load_file_session_at(path: &Path) -> Result<Session> {
    let data = match fs::read(path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Err(Error::NoSession),
        Err(err) => return Err(err.into()),
    };
    let stored: StoredSession = serde_json::from_slice(&data)?;
    Ok(Session {
        ponymail: stored.ponymail,
    })
}

#[cfg(unix)]
fn write_private_file(path: &Path, data: &[u8]) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(data)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn write_private_file(path: &Path, data: &[u8]) -> Result<()> {
    fs::write(path, data)?;
    Ok(())
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredSession {
    ponymail: String,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::models::Session;

    use super::{load_file_session_at, store_file_session_at};

    #[test]
    fn store_and_load_file_session() {
        let path = unique_test_path();
        let session = Session {
            ponymail: "abc-123".to_string(),
        };

        store_file_session_at(&path, &session).unwrap();
        let loaded = load_file_session_at(&path).unwrap();
        assert_eq!(loaded.ponymail, session.ponymail);

        cleanup_test_path(path);
    }

    #[cfg(unix)]
    #[test]
    fn file_session_uses_private_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let path = unique_test_path();
        let session = Session {
            ponymail: "abc-123".to_string(),
        };

        store_file_session_at(&path, &session).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        cleanup_test_path(path);
    }

    fn unique_test_path() -> PathBuf {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        let mut path = std::env::temp_dir();
        path.push(format!(
            "asfml-session-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path.push("session.json");
        path
    }

    fn cleanup_test_path(path: PathBuf) {
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}
