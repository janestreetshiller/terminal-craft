use std::io;
use std::path::{Path, PathBuf};

pub fn resolve_save(
    primary: Option<PathBuf>,
    legacy: Option<PathBuf>,
    base: &Path,
) -> io::Result<PathBuf> {
    if let Some(path) = primary.or(legacy) {
        return Ok(path);
    }
    let path = base.join("terminal-craft/world.tcrf");
    let old = base.join("tuicraft/world.tcrf");
    if !path.exists() && old.is_file() {
        std::fs::create_dir_all(path.parent().unwrap())?;
        let mut source = std::fs::File::open(old)?;
        // Never replace a newer save, even if two instances launch together.
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut dest) => {
                if let Err(e) = std::io::copy(&mut source, &mut dest).and_then(|_| dest.sync_all())
                {
                    let _ = std::fs::remove_file(&path);
                    return Err(e);
                }
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_copies_legacy_save_without_overwriting_either_world() {
        let base =
            std::env::temp_dir().join(format!("terminal-craft-migrate-{}", std::process::id()));
        std::fs::create_dir_all(base.join("tuicraft")).unwrap();
        std::fs::write(base.join("tuicraft/world.tcrf"), b"legacy").unwrap();
        let path = resolve_save(None, None, &base).unwrap();
        assert_eq!(path, base.join("terminal-craft/world.tcrf"));
        assert_eq!(std::fs::read(&path).unwrap(), b"legacy");
        std::fs::write(&path, b"newer").unwrap();
        assert_eq!(resolve_save(None, None, &base).unwrap(), path);
        assert_eq!(std::fs::read(&path).unwrap(), b"newer");
        assert_eq!(
            std::fs::read(base.join("tuicraft/world.tcrf")).unwrap(),
            b"legacy"
        );
        std::fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn explicit_save_path_has_priority_and_never_migrates() {
        let base = Path::new("/nonexistent/terminal-craft-test");
        assert_eq!(
            resolve_save(Some("new".into()), Some("old".into()), base).unwrap(),
            Path::new("new")
        );
        assert_eq!(
            resolve_save(None, Some("old".into()), base).unwrap(),
            Path::new("old")
        );
    }
}
