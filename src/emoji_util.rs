//! java: EmojiUtil.cleanStringForRegex and EmojiFontRenderer.
//! getEmojiFormattedString, the font renderer's scan (replace every emoji
//! code with U+2603 and map each snowman index back to its emoji), is
//! ported 1:1 here; the GL texture drawing itself is a seam (the mod API
//! has no font-renderer hook yet, tracked in the PR and the upstream gap).

use crate::emoji::Emoji;
use crate::Config;

/// java: LinkedHashMap — put replaces in place, putAll appends at the end,
/// iteration follows insertion order.
#[derive(Default, Clone, Debug)]
pub struct LinkedIndexMap(Vec<(i64, Emoji)>);

impl LinkedIndexMap {
    /// java: LinkedHashMap.put (the original's Integer keys keep -1 for
    /// indexOf misses)
    pub fn put(&mut self, key: i64, value: Emoji) {
        if let Some(slot) = self.0.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = value;
        } else {
            self.0.push((key, value));
        }
    }
    /// java: LinkedHashMap.remove
    pub fn remove(&mut self, key: i64) {
        self.0.retain(|(k, _)| *k != key);
    }
    /// java: keySet in insertion order
    pub fn keys(&self) -> Vec<i64> {
        self.0.iter().map(|(k, _)| *k).collect()
    }
    /// java: putAll (appends at the end, like the original's clean map)
    pub fn put_all(&mut self, other: LinkedIndexMap) {
        for (key, value) in other.0 {
            self.put(key, value);
        }
    }
    pub fn take(&mut self, key: i64) -> Emoji {
        let pos = self.0.iter().position(|(k, _)| *k == key).unwrap();
        self.0.remove(pos).1
    }
}

/// java: EmojiUtil.cleanStringForRegex — every backslash in the original is
/// doubled through replaceAll, so ) ( | * each gain exactly one backslash.
pub fn clean_string_for_regex(string: &str) -> String {
    string
        .replace(')', r"\)")
        .replace('(', r"\(")
        .replace('|', r"\|")
        .replace('*', r"\*")
}

/// java: ChatFormatting.stripFormatting — StringDecomposer.iterateFormatted
/// with a sink that skips only the § itself. A § always leaves the output;
/// the code character after it is consumed either way: a valid formatting
/// code (hex digit or k l m n o r, either case) switches the style, an
/// invalid one resets to the default style; a trailing § stops the scan.
fn is_formatting_code(c: char) -> bool {
    c.is_ascii_hexdigit() || "klmnorKLMNOR".contains(c)
}

pub fn strip_formatting(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '§' {
            match chars.peek() {
                None => break,
                Some(&next) => {
                    if !is_formatting_code(next) {
                        // java: an invalid code resets the style and the
                        // loop consumes the code character with ++k
                    }
                    chars.next();
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// java: EmojiFontRenderer.getEmojiFormattedString — the renderer's scan.
/// The indices and shift arithmetic are the original's: each emoji's regex
/// runs over the formatting-stripped copy case-sensitively, every hit maps
/// the index of its text in the original (case-sensitive; a miss maps -1,
/// the original's indexOf quirk) to the emoji, later indexes shift down by
/// the replaced length minus the one-character snowman, and both copies
/// replace the hit with U+2603 (case-insensitive on the original text,
/// case-sensitive on the stripped copy).
pub fn get_emoji_formatted_string(
    text: &str,
    config: &Config,
    emoji_list: &[Emoji],
) -> (String, LinkedIndexMap) {
    let mut emojis = LinkedIndexMap::default();
    let mut owned_text = text.to_string();
    let mut unformatted_text = strip_formatting(text);
    if config.render_emoji && !text.is_empty() && !unformatted_text.is_empty() {
        for emoji in emoji_list {
            let pattern = emoji.regex_pattern();
            let mut cursor = 0usize;
            while let Some(found) = pattern.find_at(&unformatted_text, cursor) {
                let emoji_text = found.as_str().to_string();
                // java: String.indexOf counts characters, not bytes
                let index = owned_text
                    .find(&emoji_text)
                    .map(|byte| owned_text[..byte].chars().count() as i64)
                    .unwrap_or(-1);
                emojis.put(index, emoji.clone());
                // java: everything past the hit shifts down by
                // (its length - 1), the snowman replacing the code
                let mut clean = LinkedIndexMap::default();
                for key in emojis.keys() {
                    if key > index {
                        let e = emojis.take(key);
                        clean.put(key - emoji_text.chars().count() as i64 + 1, e);
                    }
                }
                emojis.put_all(clean);
                unformatted_text = unformatted_text.replacen(&emoji_text, "☃", 1);
                if let Some(pos) = find_case_insensitive(&owned_text, &emoji_text) {
                    owned_text.replace_range(pos..pos + emoji_text.chars().count(), "☃");
                }
                cursor = 0;
            }
        }
    }
    (owned_text, emojis)
}

/// java: "(?i)" + Pattern.quote — the first case-insensitive occurrence,
/// compared whole character like the Java pattern does.
fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    let nee: Vec<char> = needle.chars().collect();
    if nee.is_empty() || haystack.chars().count() < nee.len() {
        return None;
    }
    // the offset is in BYTES, so replace_range can use it directly
    let hay: Vec<(usize, char)> = haystack.char_indices().collect();
    for start in 0..=(hay.len() - nee.len()) {
        if hay[start..start + nee.len()]
            .iter()
            .zip(nee.iter())
            .all(|((_, a), b)| a.eq_ignore_ascii_case(b))
        {
            return Some(hay[start].0);
        }
    }
    None
}
