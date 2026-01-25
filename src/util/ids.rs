/// ID generation utilities.
///
/// Generates unique, URL-safe identifiers for terminals.

use nanoid::nanoid;

/// URL-safe alphabet: alphanumeric only (no `-` or `~`)
const ALPHABET: [char; 62] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
    'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
    'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
];

/// Terminal ID prefix
const TERMINAL_PREFIX: &str = "term-";

/// Length of the random suffix
const TERMINAL_ID_LEN: usize = 12;

/// Generate a new terminal ID with "term-" prefix.
///
/// Format: `term-XXXXXXXXXXXX` (12 alphanumeric characters)
pub fn new_terminal_id() -> String {
    format!("{}{}", TERMINAL_PREFIX, nanoid!(TERMINAL_ID_LEN, &ALPHABET))
}

/// Check if a string is a valid terminal ID.
pub fn is_valid_terminal_id(id: &str) -> bool {
    if !id.starts_with(TERMINAL_PREFIX) {
        return false;
    }
    let suffix = &id[TERMINAL_PREFIX.len()..];
    suffix.len() == TERMINAL_ID_LEN && suffix.chars().all(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_id_has_correct_prefix() {
        let id = new_terminal_id();
        assert!(id.starts_with("term-"), "ID should start with 'term-': {}", id);
    }

    #[test]
    fn terminal_id_has_correct_length() {
        let id = new_terminal_id();
        // "term-" (5) + 12 random chars = 17
        assert_eq!(id.len(), 17, "ID should be 17 chars: {}", id);
    }

    #[test]
    fn terminal_id_suffix_is_alphanumeric() {
        let id = new_terminal_id();
        let suffix = &id[5..]; // Skip "term-"
        assert!(
            suffix.chars().all(|c| c.is_ascii_alphanumeric()),
            "Suffix should be alphanumeric: {}",
            suffix
        );
    }

    #[test]
    fn terminal_ids_are_unique() {
        let id1 = new_terminal_id();
        let id2 = new_terminal_id();
        assert_ne!(id1, id2, "IDs should be unique");
    }

    #[test]
    fn is_valid_terminal_id_accepts_valid() {
        let id = new_terminal_id();
        assert!(is_valid_terminal_id(&id));
    }

    #[test]
    fn is_valid_terminal_id_rejects_wrong_prefix() {
        assert!(!is_valid_terminal_id("foo-123456789012"));
    }

    #[test]
    fn is_valid_terminal_id_rejects_wrong_length() {
        assert!(!is_valid_terminal_id("term-short"));
        assert!(!is_valid_terminal_id("term-waytoolongtobevalid"));
    }

    #[test]
    fn is_valid_terminal_id_rejects_invalid_chars() {
        assert!(!is_valid_terminal_id("term-12345678901!")); // special char
        assert!(!is_valid_terminal_id("term-12345678901-")); // dash
    }
}
