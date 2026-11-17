//! Guardado mediante archivo temporal en el mismo directorio y reemplazo final.
use std::{fs::{self, OpenOptions}, io::{self, Write}, path::Path, sync::atomic::{AtomicU64, Ordering}};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn write_atomic(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    let path = path.as_ref();
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = path.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "La ruta debe indicar un archivo"))?;
    let mut temporary_name = name.to_os_string();
    temporary_name.push(format!(".{}.{}.tmp", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed)));
    let temporary = parent.join(temporary_name);
    let mut file = OpenOptions::new().write(true).create_new(true).open(&temporary)?;
    let result = (|| {
        file.write_all(contents.as_ref())?;
        if let Ok(metadata) = fs::metadata(path) {
            file.set_permissions(metadata.permissions())?;
        }
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() { let _ = fs::remove_file(&temporary); }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replaces_complete_document_and_cleans_up_failed_save() {
        let dir = std::env::temp_dir().join(format!("soryntel-save-{}-{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("proyecto.json");
        write_atomic(&path, "{\"estado\":1}").unwrap();
        write_atomic(&path, "{\"estado\":2}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"estado\":2}");
        let directory_target = dir.join("carpeta");
        fs::create_dir(&directory_target).unwrap();
        assert!(write_atomic(&directory_target, "datos").is_err());
        assert!(directory_target.is_dir());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        fs::remove_dir_all(dir).unwrap();
    }
}
