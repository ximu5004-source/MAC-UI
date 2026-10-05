//! Safe, non-overwriting creation of a single file in a caller-owned directory.
use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
};

pub fn valid_name(name: &str) -> bool {
    if name.is_empty()
        || name != name.trim()
        || name.ends_with('.')
        || name.encode_utf16().count() > 240
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return false;
    }
    let stem = name.split('.').next().unwrap_or_default().to_uppercase();
    !matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) && !(stem.len() == 4
        && (stem.starts_with("COM") || stem.starts_with("LPT"))
        && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        && !["COM¹", "COM²", "COM³", "LPT¹", "LPT²", "LPT³"].contains(&stem.as_str())
}

pub fn valid_extension(extension: &str) -> bool {
    extension.starts_with('.')
        && (2..=32).contains(&extension.len())
        && extension[1..]
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

pub fn create_unique_file(
    directory: &Path,
    base_name: &str,
    extension: &str,
    content: &[u8],
) -> io::Result<PathBuf> {
    if !valid_name(base_name) || !valid_extension(extension) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid file name or extension",
        ));
    }
    for number in 1..=10_000 {
        let name = if number == 1 {
            format!("{base_name}{extension}")
        } else {
            format!("{base_name} ({number}){extension}")
        };
        if name.encode_utf16().count() > 255 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "File name is too long",
            ));
        }
        let path = directory.join(name);
        // A symlink, directory or a concurrent creator must never be overwritten.
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(error) = file.write_all(content).and_then(|_| file.sync_all()) {
                    drop(file);
                    let _ = std::fs::remove_file(&path); // Only the file just created here.
                    return Err(error);
                }
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists || path.exists() => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Too many files with the same name",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            static ID: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "seelen-new-file-test-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                ID.fetch_add(1, Ordering::Relaxed),
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            // This exact, freshly-created temporary directory is owned by this test.
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn rejects_invalid_windows_names_and_path_traversal() {
        for name in [
            "",
            ".",
            "..",
            "../escape",
            "a\\b",
            "a:b",
            "x\0",
            "x.",
            "x ",
            "CON.txt",
            "NUL",
            "com1",
            "LPT9",
            "COM¹",
        ] {
            assert!(!valid_name(name), "{name:?}");
        }
        for name in ["新建文本文档", "notes.v2", "COM10", "中文 😀"] {
            assert!(valid_name(name));
        }
        for ext in ["", "txt", ".", ".txt/evil", ".a:b", ".txt.exe", ".中文"] {
            assert!(!valid_extension(ext));
        }
        assert!(valid_extension(".library-ms"));
    }

    #[test]
    fn creates_empty_text_and_copies_template_bytes_exactly() {
        let directory = TestDirectory::new();
        let text = create_unique_file(&directory.0, "中文", ".txt", b"").unwrap();
        assert_eq!(std::fs::metadata(text).unwrap().len(), 0);
        let data = b"PK\x05\x06\0\0\x00\xff";
        let file = create_unique_file(&directory.0, "Archive", ".zip", data).unwrap();
        assert_eq!(std::fs::read(file).unwrap(), data);
    }

    #[test]
    fn duplicate_file_and_directory_are_never_overwritten() {
        let directory = TestDirectory::new();
        let first = create_unique_file(&directory.0, "Notes", ".txt", b"original").unwrap();
        std::fs::create_dir(directory.0.join("Notes (2).txt")).unwrap();
        let next = create_unique_file(&directory.0, "Notes", ".txt", b"new").unwrap();
        assert_eq!(next.file_name().unwrap(), "Notes (3).txt");
        assert_eq!(std::fs::read(first).unwrap(), b"original");
    }

    #[test]
    fn concurrent_creation_uses_distinct_names() {
        let directory = TestDirectory::new();
        let paths = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        create_unique_file(&directory.0, "Concurrent", ".txt", b"safe").unwrap()
                    })
                })
                .collect();
            threads
                .into_iter()
                .map(|thread| thread.join().unwrap())
                .collect::<std::collections::HashSet<_>>()
        });
        assert_eq!(paths.len(), 8);
    }
}
