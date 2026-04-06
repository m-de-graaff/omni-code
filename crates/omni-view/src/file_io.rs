//! File I/O: encoding-aware reading and atomic writing.
//!
//! All file operations go through this module to ensure consistent
//! encoding detection, line-ending handling, and atomic writes.

use std::io::{BufWriter, Write};
use std::path::Path;

use encoding_rs::Encoding;
use omni_core::{LineEnding, Text};

/// Errors from file I/O operations.
#[derive(Debug, thiserror::Error)]
pub enum FileIoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("file has no path (use save_as)")]
    NoPath,
    #[error("encoding error: text contains characters not representable in {0}")]
    Encoding(String),
    #[error("persist error: {0}")]
    Persist(#[from] tempfile::PersistError),
}

/// Read a file from disk, detecting its encoding.
///
/// Returns the decoded UTF-8 content, the detected encoding, and the
/// raw file size in bytes.
///
/// Encoding detection strategy:
/// 1. Check for BOM (UTF-8, UTF-16 LE/BE)
/// 2. Try UTF-8 decode — if no errors, use UTF-8
/// 3. Fall back to Windows-1252 (superset of Latin-1, never fails)
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn read_file(path: &Path) -> Result<(String, &'static Encoding, usize), FileIoError> {
    let bytes = std::fs::read(path)?;
    let file_size = bytes.len();
    let (encoding, content) = decode_bytes(&bytes);
    Ok((content, encoding, file_size))
}

/// Write document content to a file atomically.
///
/// 1. Creates a temp file in the same directory (ensures same filesystem).
/// 2. Normalizes line endings to the document's style.
/// 3. Re-encodes from UTF-8 to the target encoding.
/// 4. Atomically renames temp → target.
///
/// # Errors
///
/// Returns an error if the file cannot be written or renamed.
pub fn write_file(
    path: &Path,
    text: &Text,
    encoding: &'static Encoding,
    line_ending: LineEnding,
) -> Result<(), FileIoError> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = tempfile::NamedTempFile::new_in(dir)?;
    {
        let mut writer = BufWriter::new(&tmp);
        for chunk in text.rope().chunks() {
            let normalized = normalize_line_endings(chunk, line_ending);
            let encoded = encode_str(&normalized, encoding)?;
            writer.write_all(&encoded)?;
        }
        writer.flush()?;
    }
    tmp.persist(path)?;
    Ok(())
}

/// Detect encoding from raw bytes and decode to UTF-8.
fn decode_bytes(bytes: &[u8]) -> (&'static Encoding, String) {
    // 1. Check for BOM
    if let Some((encoding, _bom_len)) = Encoding::for_bom(bytes) {
        let (cow, _, _) = encoding.decode(bytes);
        return (encoding, cow.into_owned());
    }

    // 2. Try UTF-8
    let (cow, _, had_errors) = encoding_rs::UTF_8.decode(bytes);
    if !had_errors {
        return (encoding_rs::UTF_8, cow.into_owned());
    }

    // 3. Fall back to Windows-1252 (never fails)
    let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    (encoding_rs::WINDOWS_1252, cow.into_owned())
}

/// Normalize line endings in a text chunk.
///
/// Ropey stores `\n` internally. When the document uses CRLF,
/// we replace `\n` with `\r\n` on output.
fn normalize_line_endings(text: &str, line_ending: LineEnding) -> String {
    match line_ending {
        LineEnding::Lf => text.to_string(),
        LineEnding::CrLf => {
            // Replace lone \n with \r\n
            let mut result = String::with_capacity(text.len() + text.len() / 20);
            for ch in text.chars() {
                if ch == '\n' {
                    result.push_str("\r\n");
                } else {
                    result.push(ch);
                }
            }
            result
        }
    }
}

/// Encode a UTF-8 string to the target encoding.
fn encode_str(text: &str, encoding: &'static Encoding) -> Result<Vec<u8>, FileIoError> {
    if encoding == encoding_rs::UTF_8 {
        return Ok(text.as_bytes().to_vec());
    }
    let (cow, _, had_errors) = encoding.encode(text);
    if had_errors {
        return Err(FileIoError::Encoding(encoding.name().to_string()));
    }
    Ok(cow.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn read_utf8_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "hello\nworld").unwrap();

        let (content, encoding, size) = read_file(&path).unwrap();
        assert_eq!(content, "hello\nworld");
        assert_eq!(encoding, encoding_rs::UTF_8);
        assert_eq!(size, 11);
    }

    #[test]
    fn read_utf8_bom_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bom.txt");
        let mut bytes = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
        bytes.extend_from_slice(b"hello");
        fs::write(&path, &bytes).unwrap();

        let (content, encoding, _) = read_file(&path).unwrap();
        assert!(content.contains("hello"));
        assert_eq!(encoding, encoding_rs::UTF_8);
    }

    #[test]
    fn read_latin1_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("latin1.txt");
        // 0xE9 = é in Latin-1/Windows-1252 (invalid UTF-8)
        fs::write(&path, &[0x68, 0x65, 0x6C, 0x6C, 0xE9]).unwrap();

        let (content, encoding, _) = read_file(&path).unwrap();
        assert!(content.contains("hell"));
        assert_eq!(encoding, encoding_rs::WINDOWS_1252);
    }

    #[test]
    fn write_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");
        let text = Text::from("hello\nworld");

        write_file(&path, &text, encoding_rs::UTF_8, LineEnding::Lf).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello\nworld");
    }

    #[test]
    fn write_preserves_crlf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("crlf.txt");
        let text = Text::from("hello\nworld\n");

        write_file(&path, &text, encoding_rs::UTF_8, LineEnding::CrLf).unwrap();

        let bytes = fs::read(&path).unwrap();
        let content = String::from_utf8(bytes).unwrap();
        assert_eq!(content, "hello\r\nworld\r\n");
    }

    #[test]
    fn write_atomic_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.txt");
        assert!(!path.exists());

        let text = Text::from("new file");
        write_file(&path, &text, encoding_rs::UTF_8, LineEnding::Lf).unwrap();

        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "new file");
    }
}
