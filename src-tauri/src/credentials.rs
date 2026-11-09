//! 旧明文凭据仅允许迁入系统钥匙串，不再新增明文回退文件。
use std::path::Path;
use std::sync::Mutex;

static MIGRATION_LOCK: Mutex<()> = Mutex::new(());

pub fn migrate(path: &Path, mut save: impl FnMut(&str, &str) -> Result<(), String>) -> Result<(), String> {
    let _guard = MIGRATION_LOCK.lock().map_err(|_| "The credential migration lock is unavailable.")?;
    if !path.exists() { return Ok(()); }
    let bytes = std::fs::read(path).map_err(|_| "Could not read the old credentials file.")?;
    let values: std::collections::BTreeMap<String, String> = serde_json::from_slice(&bytes)
        .map_err(|_| "The old credentials file has an invalid format. The file was preserved.")?;
    for (key, value) in values { save(&key, &value)?; }
    // 所有条目已安全持久化后，才移除旧文件；失败时保留以便恢复。
    std::fs::remove_file(path).map_err(|_| "Credentials were migrated to the keychain, but the old plain-text file could not be removed. Check its permissions.".into())
}

pub fn migrate_to_keyring(path: &Path, service: &str, fixed_key: Option<&str>) -> Result<(), String> {
    migrate(path, |key, value| {
        let entry = keyring::Entry::new(service, fixed_key.unwrap_or(key)).map_err(|_| "The system keychain is unavailable.")?;
        match entry.get_password() {
            Ok(_) => Ok(()), // 不用旧文件覆盖后来保存的新凭据。
            Err(keyring::Error::NoEntry) => entry.set_password(value).map_err(|_| "Failed to migrate the old credentials. The file was preserved.".into()),
            Err(_) => Err("Could not read the keychain. The old credentials file was preserved.".into()),
        }
    })
}

/// 供应商端点仅接受 http/https；凭据一律走请求头，禁止内嵌在 URL 或携带 # 片段。
pub fn validate_endpoint(url: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(url).map_err(|_| "The server address is invalid.")?;
    if !(url.scheme() == "http" || url.scheme() == "https")
        || !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err("The server address must use HTTP or HTTPS and must not include credentials or a # fragment.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn security_migration_never_discards_unsaved_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys.json");
        std::fs::write(&path, r#"{"a":"测试值","b":"另一测试值"}"#).unwrap();
        assert!(migrate(&path, |_, _| Err("不可用".into())).is_err());
        assert!(path.exists());
        let mut saved = Vec::new();
        migrate(&path, |k, v| { saved.push((k.to_string(), v.to_string())); Ok(()) }).unwrap();
        assert_eq!(saved.len(), 2);
        assert!(!path.exists());
    }
    #[test]
    fn endpoint_accepts_http_and_https_only() {
        for url in [
            "https://example.com/v1",
            "http://example.com/v1",
            "http://192.168.1.10:8000/v1",
            "http://127.0.0.1:11434/v1",
            "http://[::1]:11434/v1",
        ] {
            assert!(validate_endpoint(url).is_ok(), "{url}");
        }
        for url in [
            "ftp://example.com/v1",
            "file:///secret",
            "https://user:pass@example.com/",
            "http://user@example.com/v1",
            "https://example.com/v1#frag",
        ] {
            assert!(validate_endpoint(url).is_err(), "{url}");
        }
    }
}
