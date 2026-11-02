//! UTF-8 byte offset ↔ UTF-16 code-unit offset for one file's text.
//!
//! `kul-core` and the language service count UTF-8 bytes; JavaScript strings
//! and CodeMirror count UTF-16 code units. The [`Workspace`](crate::Workspace)
//! converts every offset crossing the boundary through one of these, so no
//! other code needs to know the two units differ.

/// Offset converter for one text. Only the characters whose UTF-8 and UTF-16
/// lengths differ (everything outside ASCII) are recorded, so ASCII text
/// costs nothing and a lookup is a binary search.
#[derive(Debug, Clone)]
pub(crate) struct Utf16Index {
    /// The non-ASCII characters, in text order.
    wide: Vec<Wide>,
    bytes: usize,
    units: usize,
}

#[derive(Debug, Clone, Copy)]
struct Wide {
    /// Byte offset of the character.
    byte: usize,
    /// UTF-16 offset of the character.
    unit: usize,
    /// Its UTF-8 length.
    bytes: usize,
    /// Its UTF-16 length.
    units: usize,
}

impl Utf16Index {
    pub(crate) fn new(text: &str) -> Self {
        let mut wide = Vec::new();
        let mut unit = 0;
        for (byte, ch) in text.char_indices() {
            if !ch.is_ascii() {
                wide.push(Wide {
                    byte,
                    unit,
                    bytes: ch.len_utf8(),
                    units: ch.len_utf16(),
                });
            }
            unit += ch.len_utf16();
        }
        Self {
            wide,
            bytes: text.len(),
            units: unit,
        }
    }

    /// The UTF-16 offset of byte offset `byte`. Past the end clamps to the
    /// end; inside a character snaps to the character's start.
    pub(crate) fn to_utf16(&self, byte: usize) -> usize {
        let byte = byte.min(self.bytes);
        let before = self.wide.partition_point(|w| w.byte < byte);
        match before.checked_sub(1).map(|i| self.wide[i]) {
            None => byte,
            Some(w) if byte < w.byte + w.bytes => w.unit,
            Some(w) => w.unit + w.units + (byte - w.byte - w.bytes),
        }
    }

    /// The byte offset of UTF-16 offset `unit`. Past the end clamps to the
    /// end; inside a surrogate pair snaps to the character's start.
    pub(crate) fn to_byte(&self, unit: usize) -> usize {
        let unit = unit.min(self.units);
        let before = self.wide.partition_point(|w| w.unit < unit);
        match before.checked_sub(1).map(|i| self.wide[i]) {
            None => unit,
            Some(w) if unit < w.unit + w.units => w.byte,
            Some(w) => w.byte + w.bytes + (unit - w.unit - w.units),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every char boundary converts both ways and agrees with Rust's own
    /// UTF-16 count of the prefix.
    fn round_trips(text: &str) {
        let index = Utf16Index::new(text);
        for (byte, _) in text.char_indices().chain([(text.len(), ' ')]) {
            let unit = text[..byte].encode_utf16().count();
            assert_eq!(index.to_utf16(byte), unit, "byte {byte} of {text:?}");
            assert_eq!(index.to_byte(unit), byte, "unit {unit} of {text:?}");
        }
    }

    #[test]
    fn ascii_is_the_identity() {
        let index = Utf16Index::new("hello\nworld\n");
        assert_eq!(index.to_utf16(7), 7);
        assert_eq!(index.to_byte(7), 7);
        round_trips("hello\nworld\n");
    }

    #[test]
    fn emoji_is_four_bytes_and_two_units() {
        // 🎉 is U+1F389: 4 bytes in UTF-8, a surrogate pair in UTF-16.
        let index = Utf16Index::new("a🎉b");
        assert_eq!(index.to_utf16(1), 1);
        assert_eq!(index.to_utf16(5), 3);
        assert_eq!(index.to_utf16(6), 4);
        assert_eq!(index.to_byte(3), 5);
        round_trips("a🎉b");
    }

    #[test]
    fn accented_latin_is_two_bytes_and_one_unit() {
        let index = Utf16Index::new("café");
        assert_eq!(index.to_utf16(5), 4);
        assert_eq!(index.to_byte(4), 5);
    }

    #[test]
    fn gujarati_and_devanagari_are_three_bytes_and_one_unit() {
        round_trips("person ભાવના name:\"ભાવના પટેલ\"\nperson अनिता # टिप्पणी\n");
        let index = Utf16Index::new("ક");
        assert_eq!(index.to_utf16(3), 1);
        assert_eq!(index.to_byte(1), 3);
    }

    #[test]
    fn mixed_scripts_round_trip() {
        round_trips("# 👨‍👩‍👧 family\r\nperson x name:\"Zoë 李 ભાઈ 🎉\" gender:male\n");
    }

    #[test]
    fn offsets_inside_a_character_snap_to_its_start() {
        let index = Utf16Index::new("a🎉b");
        assert_eq!(index.to_utf16(2), 1);
        assert_eq!(index.to_utf16(4), 1);
        assert_eq!(index.to_byte(2), 1);
    }

    #[test]
    fn offsets_past_the_end_clamp() {
        let index = Utf16Index::new("a🎉");
        assert_eq!(index.to_utf16(99), 3);
        assert_eq!(index.to_byte(99), 5);
    }

    #[test]
    fn empty_text() {
        let index = Utf16Index::new("");
        assert_eq!(index.to_utf16(0), 0);
        assert_eq!(index.to_byte(5), 0);
    }
}
