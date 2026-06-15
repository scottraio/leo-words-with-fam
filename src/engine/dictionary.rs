//! Word list used to validate plays. Backed by an in-memory `HashSet` loaded
//! once at startup (the ENABLE list is ~172k words; lookups are O(1)).

use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct Dictionary {
    words: HashSet<String>,
}

impl Dictionary {
    /// Build from an iterator of words (used by tests and loaders).
    pub fn from_words<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let words = iter
            .into_iter()
            .map(|w| w.as_ref().trim().to_ascii_uppercase())
            .filter(|w| !w.is_empty())
            .collect();
        Dictionary { words }
    }

    /// Load a newline-delimited word list from disk (one word per line).
    pub fn load_from_file(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(Self::from_words(contents.lines()))
    }

    /// Case-insensitive membership test.
    pub fn contains(&self, word: &str) -> bool {
        self.words.contains(&word.trim().to_ascii_uppercase())
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn membership_is_case_insensitive() {
        let dict = Dictionary::from_words(["hello", "WORLD", "Quiz"]);
        assert!(dict.contains("HELLO"));
        assert!(dict.contains("hello"));
        assert!(dict.contains("world"));
        assert!(dict.contains("quiz"));
        assert!(!dict.contains("zzz"));
        assert_eq!(dict.len(), 3);
    }

    #[test]
    fn blank_lines_ignored() {
        let dict = Dictionary::from_words(["cat", "", "  ", "dog"]);
        assert_eq!(dict.len(), 2);
    }
}
