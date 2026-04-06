//! Line ending detection and representation.

/// The line ending style used in a document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LineEnding {
    /// Unix / macOS: `\n`
    #[default]
    Lf,
    /// Windows: `\r\n`
    CrLf,
}

impl LineEnding {
    /// Detect the line ending style from a text sample.
    ///
    /// Scans the first 8 KB for `\r\n`; if found, returns [`CrLf`](Self::CrLf),
    /// otherwise [`Lf`](Self::Lf).
    #[must_use]
    pub fn detect(text: &str) -> Self {
        let sample = &text[..text.len().min(8192)];
        if sample.contains("\r\n") { Self::CrLf } else { Self::Lf }
    }

    /// The string representation of this line ending.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}

impl std::fmt::Display for LineEnding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lf => f.write_str("LF"),
            Self::CrLf => f.write_str("CRLF"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_lf() {
        assert_eq!(LineEnding::detect("hello\nworld\n"), LineEnding::Lf);
    }

    #[test]
    fn detects_crlf() {
        assert_eq!(LineEnding::detect("hello\r\nworld\r\n"), LineEnding::CrLf);
    }

    #[test]
    fn detects_lf_for_empty() {
        assert_eq!(LineEnding::detect(""), LineEnding::Lf);
    }

    #[test]
    fn detects_lf_for_no_newlines() {
        assert_eq!(LineEnding::detect("hello world"), LineEnding::Lf);
    }

    #[test]
    fn as_str_values() {
        assert_eq!(LineEnding::Lf.as_str(), "\n");
        assert_eq!(LineEnding::CrLf.as_str(), "\r\n");
    }
}
