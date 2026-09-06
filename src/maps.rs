//! Original bundled worlds. Each template gets a separate editable save.
use crate::{game::Game, world::World};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub struct Map {
    pub id: &'static str,
    pub title: &'static str,
    pub bytes: &'static [u8],
}
pub const MAPS: [Map; 5] = [
    Map {
        id: "foundry",
        title: "FOUNDRY",
        bytes: include_bytes!("../assets/maps/foundry.tcrf"),
    },
    Map {
        id: "switchyard",
        title: "SWITCHYARD",
        bytes: include_bytes!("../assets/maps/switchyard.tcrf"),
    },
    Map {
        id: "citadel",
        title: "CITADEL",
        bytes: include_bytes!("../assets/maps/citadel.tcrf"),
    },
    Map {
        id: "skybridge",
        title: "SKYBRIDGE",
        bytes: include_bytes!("../assets/maps/skybridge.tcrf"),
    },
    Map {
        id: "dune-outpost",
        title: "DUNE OUTPOST",
        bytes: include_bytes!("../assets/maps/dune-outpost.tcrf"),
    },
];

pub fn find(id: &str) -> io::Result<&'static Map> {
    MAPS.iter().find(|m| m.id == id).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "unknown prebuilt map; use --maps",
        )
    })
}
pub fn save_path(base: &Path, id: &str) -> io::Result<PathBuf> {
    find(id)?;
    let mut directory = base.as_os_str().to_os_string();
    directory.push(".prebuilt");
    Ok(PathBuf::from(directory).join(format!("{id}.tcrf")))
}
pub fn ensure(base: &Path, id: &str) -> io::Result<PathBuf> {
    let path = save_path(base, id)?;
    if path.try_exists()? {
        return Ok(path);
    }
    std::fs::create_dir_all(path.parent().unwrap())?;
    match export(id, &path) {
        Ok(()) => Ok(path),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(path),
        Err(e) => Err(e),
    }
}
pub fn open(base: &Path, id: &str) -> io::Result<Game> {
    let mut game = Game::load(&ensure(base, id)?)?;
    game.toast(find(id)?.title);
    Ok(game)
}
pub fn export(id: &str, path: &Path) -> io::Result<()> {
    let map = find(id)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(".prebuilt-{}-{nonce}.tmp", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        file.write_all(map.bytes)?;
        file.sync_all()?;
        // Publish only a complete file, atomically and without overwriting edits.
        std::fs::hard_link(&temporary, path)
    })();
    drop(file);
    let cleanup = std::fs::remove_file(&temporary);
    result.and(cleanup)
}
pub fn preview(id: &str) -> io::Result<World> {
    let mut bytes = find(id)?.bytes;
    let (world, _, _, _, _) = World::read(&mut bytes)?;
    Ok(world)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_templates_parse_and_corrupt_resumes_are_not_reset() {
        let root = std::env::temp_dir().join(format!(
            "terminal-craft-corrupt-map-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let base = root.join("world.tcrf");
        for map in &MAPS {
            assert!(preview(map.id).is_ok());
            let path = ensure(&base, map.id).unwrap();
            std::fs::write(&path, b"corrupt save retained").unwrap();
            assert!(open(&base, map.id).is_err());
            assert_eq!(std::fs::read(&path).unwrap(), b"corrupt save retained");
        }
        assert!(!base.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn isolated_saves_preserve_existing_edits_and_default_world() {
        let root = std::env::temp_dir().join(format!(
            "terminal-craft-maps-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let base = root.join("world.tcrf");
        std::fs::write(&base, b"existing user world").unwrap();
        for m in &MAPS {
            let path = ensure(&base, m.id).unwrap();
            assert_ne!(path, base);
            assert_eq!(std::fs::read(&path).unwrap(), m.bytes);
            std::fs::write(&path, b"edited world").unwrap();
            assert_eq!(ensure(&base, m.id).unwrap(), path);
            assert_eq!(std::fs::read(path).unwrap(), b"edited world");
        }
        assert_eq!(std::fs::read(&base).unwrap(), b"existing user world");
        assert!(save_path(&base, "../escape").is_err());
        assert!(export("foundry", &base).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
