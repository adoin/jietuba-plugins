use age::secrecy::SecretString;
use anyhow::{bail, Context, Result};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use rusqlite::{backup::Backup, Connection, DatabaseName, OpenFlags};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{self, Cursor, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

const MAX_DB: u64 = 512 * 1024 * 1024;
const MAX_IMAGE: u64 = 128 * 1024 * 1024;
const MAX_TOTAL: u64 = 4 * 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 100_000;

fn safe_image_name(name: &str) -> bool {
    let Some(id) = name.strip_suffix(".png") else {
        return false;
    };
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

fn append<R: Read, W: Write>(
    archive: &mut tar::Builder<W>,
    path: &str,
    size: u64,
    reader: R,
) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(size);
    header.set_mode(0o600);
    header.set_cksum();
    archive.append_data(&mut header, path, reader)?;
    Ok(())
}

fn referenced_images(db: &Connection) -> Result<Vec<(String, bool)>> {
    let mut stmt = db.prepare("SELECT DISTINCT c.image_id, EXISTS(SELECT 1 FROM clipboard c2 JOIN clipboard_formats f ON f.event_id=c2.id WHERE c2.image_id=c.image_id AND f.format_name COLLATE NOCASE IN ('PNG','CF_DIB','CF_DIBV5') AND length(f.data)>0) FROM clipboard c WHERE c.image_id IS NOT NULL AND c.image_id <> ''")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn snapshot(source: &Path) -> Result<(Vec<u8>, Vec<(String, bool)>)> {
    let db = Connection::open_with_flags(
        source,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let pages: u64 = db.query_row("PRAGMA page_count", [], |r| r.get(0))?;
    let page_size: u64 = db.query_row("PRAGMA page_size", [], |r| r.get(0))?;
    if pages.saturating_mul(page_size) > MAX_DB {
        bail!("database_too_large");
    }
    let mut memory = Connection::open_in_memory()?;
    {
        Backup::new(&db, &mut memory)?.run_to_completion(128, Duration::from_millis(20), None)?;
    }
    let images = referenced_images(&memory)?;
    let bytes = memory.serialize(DatabaseName::Main)?.to_vec();
    if bytes.len() as u64 > MAX_DB {
        bail!("database_too_large");
    }
    Ok((bytes, images))
}

fn export(
    db: &Path,
    images_dir: &Path,
    destination: &Path,
    password: &str,
    overwrite: bool,
    staging: Option<&Path>,
) -> Result<()> {
    if password.is_empty() {
        bail!("empty_password");
    }
    if destination
        .extension()
        .and_then(|s| s.to_str())
        .is_none_or(|s| !s.eq_ignore_ascii_case("jtbbackup"))
    {
        bail!("invalid_destination");
    }
    if destination.exists() && !overwrite {
        bail!("destination_exists");
    }
    let parent = destination.parent().context("invalid_destination")?;
    let temp_parent = if let Some(staging) = staging {
        let parent_path = parent.canonicalize()?;
        let staging_path = staging.canonicalize()?;
        if staging_path == parent_path
            || !staging_path.starts_with(&parent_path)
            || !staging.is_dir()
        {
            bail!("invalid_staging");
        }
        staging
    } else {
        parent
    };
    let (db_bytes, image_ids) = snapshot(db)?;
    if image_ids.len() > MAX_ENTRIES - 2 {
        bail!("too_many_files");
    }
    let mut output = tempfile::NamedTempFile::new_in(temp_parent)?;
    {
        let mut recipient = age::scrypt::Recipient::new(SecretString::from(password.to_owned()));
        recipient.set_work_factor(17);
        let encryptor =
            age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))?;
        let encrypted = encryptor.wrap_output(output.as_file_mut())?;
        let gzip = GzEncoder::new(encrypted, Compression::default());
        let mut archive = tar::Builder::new(gzip);
        let manifest = br#"{"format":"jietuba-backup","version":1}"#;
        append(
            &mut archive,
            "manifest.json",
            manifest.len() as u64,
            Cursor::new(manifest),
        )?;
        append(
            &mut archive,
            "clipboard.db",
            db_bytes.len() as u64,
            Cursor::new(&db_bytes),
        )?;
        let mut total = db_bytes.len() as u64;
        for (id, recoverable_from_formats) in image_ids {
            let name = format!("{id}.png");
            if !safe_image_name(&name) {
                bail!("invalid_image_id");
            }
            let path = images_dir.join(&name);
            let info = match fs::symlink_metadata(&path) {
                Ok(info) => info,
                Err(error)
                    if error.kind() == io::ErrorKind::NotFound && recoverable_from_formats =>
                {
                    continue
                }
                Err(error) => return Err(error).context("missing_image"),
            };
            if info.file_type().is_symlink() || !info.is_file() || info.len() > MAX_IMAGE {
                bail!("invalid_image");
            }
            total = total.checked_add(info.len()).context("backup_too_large")?;
            if total > MAX_TOTAL {
                bail!("backup_too_large");
            }
            append(
                &mut archive,
                &format!("images/{name}"),
                info.len(),
                File::open(path)?,
            )?;
        }
        let gzip = archive.into_inner()?;
        let encrypted = gzip.finish()?;
        encrypted.finish()?;
    }
    output.as_file().sync_all()?;
    if overwrite {
        output.persist(destination).map_err(|e| e.error)?;
    } else {
        output.persist_noclobber(destination).map_err(|e| e.error)?;
    }
    Ok(())
}

fn restore(source: &Path, staging: &Path, password: &str) -> Result<()> {
    if password.is_empty() {
        bail!("empty_password");
    }
    if !staging.is_dir() || fs::read_dir(staging)?.next().is_some() {
        bail!("invalid_staging");
    }
    let decryptor = age::Decryptor::new(File::open(source)?)?;
    if !decryptor.is_scrypt() {
        bail!("invalid_backup");
    }
    let mut identity = age::scrypt::Identity::new(SecretString::from(password.to_owned()));
    identity.set_max_work_factor(19);
    let decrypted = decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))?;
    let gzip = GzDecoder::new(decrypted);
    let mut archive = tar::Archive::new(gzip);
    let mut seen = HashSet::new();
    let mut total = 0u64;
    let mut manifest_ok = false;
    for (index, entry) in archive.entries()?.enumerate() {
        if index >= MAX_ENTRIES {
            bail!("too_many_files");
        }
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            bail!("invalid_backup");
        }
        let name = entry
            .path()?
            .to_str()
            .context("invalid_backup")?
            .to_string();
        if !seen.insert(name.to_ascii_lowercase()) {
            bail!("invalid_backup");
        }
        let size = entry.size();
        total = total.checked_add(size).context("backup_too_large")?;
        if total > MAX_TOTAL {
            bail!("backup_too_large");
        }
        if name == "manifest.json" {
            if size > 4096 {
                bail!("invalid_backup");
            }
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            let value: Value = serde_json::from_slice(&bytes)?;
            manifest_ok = value["format"] == "jietuba-backup" && value["version"] == 1;
        } else {
            let path = if name == "clipboard.db" && size <= MAX_DB {
                staging.join("clipboard.db")
            } else if let Some(file) = name.strip_prefix("images/") {
                if !safe_image_name(file) || size > MAX_IMAGE {
                    bail!("invalid_backup");
                }
                fs::create_dir_all(staging.join("images"))?;
                staging.join("images").join(file)
            } else {
                bail!("invalid_backup");
            };
            let mut output = File::options().write(true).create_new(true).open(path)?;
            if io::copy(&mut entry, &mut output)? != size {
                bail!("invalid_backup");
            }
            output.sync_all()?;
        }
    }
    // tar can finish before the encryption stream; read to the authenticated end.
    let mut gzip = archive.into_inner();
    io::copy(&mut gzip, &mut io::sink())?;
    let mut decrypted = gzip.into_inner();
    if io::copy(&mut decrypted, &mut io::sink())? != 0 {
        bail!("invalid_backup");
    }
    if !manifest_ok || !seen.contains("clipboard.db") {
        bail!("invalid_backup");
    }
    let db = Connection::open_with_flags(
        staging.join("clipboard.db"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let integrity: String = db.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        bail!("invalid_backup");
    }
    for table in ["clipboard", "clipboard_formats", "groups"] {
        let exists: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            [table],
            |r| r.get(0),
        )?;
        if !exists {
            bail!("invalid_backup");
        }
    }
    for (id, recoverable_from_formats) in referenced_images(&db)? {
        let name = format!("{id}.png");
        if !safe_image_name(&name)
            || (!recoverable_from_formats
                && !seen.contains(&format!("images/{name}").to_ascii_lowercase()))
        {
            bail!("missing_image");
        }
    }
    Ok(())
}

fn required_path(request: &Value, key: &str) -> Result<PathBuf> {
    request[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .context("invalid_request")
}

fn run(request: &Value) -> Result<()> {
    if request["api_version"] != 1 || request["type"] != "backup" {
        bail!("invalid_request");
    }
    let password = request["password"].as_str().context("invalid_request")?;
    match request["operation"].as_str() {
        Some("export") => export(
            &required_path(request, "db_path")?,
            &required_path(request, "images_dir")?,
            &required_path(request, "destination")?,
            password,
            request["overwrite"].as_bool().unwrap_or(false),
            request["staging"].as_str().map(Path::new),
        ),
        Some("restore") => restore(
            &required_path(request, "source")?,
            &required_path(request, "staging")?,
            password,
        ),
        _ => bail!("invalid_request"),
    }
}

fn main() -> io::Result<()> {
    let mut input = Vec::new();
    io::stdin().take(65537).read_to_end(&mut input)?;
    let result = serde_json::from_slice::<Value>(&input)
        .map_err(anyhow::Error::from)
        .and_then(|request| run(&request));
    let response = match result {
        Ok(()) => json!({"api_version":1,"ok":true}),
        Err(error) => {
            let message = error.to_string();
            let code = match message.as_str() {
                "missing_image" | "database_too_large" | "backup_too_large" | "too_many_files"
                | "destination_exists" => message,
                _ => "failed".to_string(),
            };
            json!({"api_version":1,"ok":false,"error":code})
        }
    };
    serde_json::to_writer(io::stdout().lock(), &response)?;
    io::stdout().flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encrypted_backup_round_trip_and_wrong_password() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db_path = dir.path().join("source.db");
        let db = Connection::open(&db_path)?;
        db.pragma_update(None, "journal_mode", "WAL")?;
        db.execute_batch("CREATE TABLE clipboard(id INTEGER PRIMARY KEY, content TEXT, image_id TEXT); CREATE TABLE clipboard_formats(event_id INTEGER, format_name TEXT, data BLOB); CREATE TABLE groups(id INTEGER); CREATE TABLE clipboard_metadata(item_id INTEGER,plugin_id TEXT,data TEXT); INSERT INTO clipboard VALUES(1,'secret text','abcd'); INSERT INTO clipboard VALUES(2,'raw image','missing'); INSERT INTO clipboard_formats VALUES(2,'PNG',X'0102'); INSERT INTO clipboard_metadata VALUES(1,'notes-tags','note value');")?;
        let images = dir.path().join("source-images");
        fs::create_dir(&images)?;
        fs::write(images.join("abcd.png"), b"image data")?;
        let backup = dir.path().join("archive.jtbbackup");
        export(&db_path, &images, &backup, "correct horse", false, None)?;
        let ciphertext = fs::read(&backup)?;
        assert!(!ciphertext
            .windows(b"secret text".len())
            .any(|w| w == b"secret text"));
        assert!(export(&db_path, &images, &backup, "correct horse", false, None).is_err());
        let wrong = dir.path().join("wrong");
        fs::create_dir(&wrong)?;
        assert!(restore(&backup, &wrong, "wrong password").is_err());
        assert!(fs::read_dir(&wrong)?.next().is_none());
        let output = dir.path().join("restored");
        fs::create_dir(&output)?;
        restore(&backup, &output, "correct horse")?;
        let restored = Connection::open(output.join("clipboard.db"))?;
        assert_eq!(
            restored.query_row("SELECT content FROM clipboard WHERE id=1", [], |r| r
                .get::<_, String>(0))?,
            "secret text"
        );
        assert_eq!(fs::read(output.join("images/abcd.png"))?, b"image data");
        assert_eq!(
            restored.query_row(
                "SELECT data FROM clipboard_metadata WHERE item_id=1",
                [],
                |r| r.get::<_, String>(0)
            )?,
            "note value"
        );
        let broken = dir.path().join("truncated.jtbbackup");
        fs::write(&broken, &ciphertext[..ciphertext.len() - 16])?;
        let corrupt_output = dir.path().join("corrupt");
        fs::create_dir(&corrupt_output)?;
        assert!(restore(&broken, &corrupt_output, "correct horse").is_err());
        Ok(())
    }
    #[test]
    fn rejects_unsafe_image_paths() {
        assert!(!safe_image_name("../secret.png"));
        assert!(!safe_image_name("foo/bar.png"));
        assert!(safe_image_name("abc123.png"));
    }

    #[test]
    fn rejects_path_traversal_even_in_authenticated_archives() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mut recipient = age::scrypt::Recipient::new(SecretString::from("test".to_owned()));
        recipient.set_work_factor(2);
        let encryptor =
            age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))?;
        let encrypted = encryptor.wrap_output(Vec::new())?;
        let gzip = GzEncoder::new(encrypted, Compression::fast());
        let mut archive = tar::Builder::new(gzip);
        let mut header = tar::Header::new_gnu();
        header.set_size(3);
        header.set_mode(0o600);
        header.as_mut_bytes()[..9].copy_from_slice(b"../escape");
        header.set_cksum();
        archive.append(&header, &b"bad"[..])?;
        let encrypted = archive.into_inner()?.finish()?.finish()?;
        let source = dir.path().join("unsafe.jtbbackup");
        fs::write(&source, encrypted)?;
        let staging = dir.path().join("staging");
        fs::create_dir(&staging)?;
        assert!(restore(&source, &staging, "test").is_err());
        assert!(!dir.path().join("escape").exists());
        Ok(())
    }
}
