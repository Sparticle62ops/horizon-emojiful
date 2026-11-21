//! java: api/Emoji.java. The fields, the regex builders, test and
//! getShorterString are the original's; the texture, cache and download
//! machinery ride the render seam (the mod API has no font hook yet).

use crate::emoji_util::clean_string_for_regex;

#[derive(Clone, Debug, PartialEq)]
pub struct Emoji {
    pub name: String,
    pub strings: Vec<String>,
    pub texts: Vec<String>,
    pub location: String,
    /// java: version, default 1
    pub version: i32,
    /// java: sort, default 0
    pub sort: i32,
    /// java: worldBased, default false
    pub world_based: bool,
}

impl Emoji {
    pub fn new(name: String, strings: Vec<String>, texts: Vec<String>, location: String) -> Self {
        Emoji {
            name,
            strings,
            texts,
            location,
            version: 1,
            sort: 0,
            world_based: false,
        }
    }

    /// java: test(String) — equalsIgnoreCase against every string.
    pub fn test(&self, s: &str) -> bool {
        self.strings.iter().any(|text| s.eq_ignore_ascii_case(text))
    }

    /// java: getShorterString — the shortest string, strings.get(0) as the
    /// start.
    pub fn get_shorter_string(&self) -> String {
        let mut short = self
            .strings
            .first()
            .cloned()
            .unwrap_or_default();
        for string in &self.strings {
            if string.chars().count() < short.chars().count() {
                short = string.clone();
            }
        }
        short
    }

    /// java: getRegexString — \b wraps a string whose first or last lower
    /// cased character is an ASCII letter, then each string passes through
    /// cleanStringForRegex and they join with |.
    pub fn get_regex_string(&self) -> String {
        let mut processed = Vec::new();
        for string in &self.strings {
            let mut s = string.clone();
            if let Some(last) = string.to_lowercase().chars().next_back() {
                if last.is_ascii_lowercase() {
                    s.push_str("\\b");
                }
            }
            if let Some(first) = string.to_lowercase().chars().next() {
                if first.is_ascii_lowercase() {
                    s = format!("\\b{}", s);
                }
            }
            processed.push(clean_string_for_regex(&s));
        }
        processed.join("|")
    }

    /// java: getTextRegex — the processed texts joined with | inside the
    /// original's lookarounds; kept as a string 1:1 (the Rust regex crate
    /// has no lookaround, and this pattern only rides the render seam).
    pub fn get_text_regex(&self) -> String {
        let processed: Vec<String> = self
            .texts
            .iter()
            .map(|string| clean_string_for_regex(string))
            .collect();
        format!("(?<=^|\\s)({})(?=$|\\s)", processed.join("|"))
    }

    /// The compiled java getRegex() under the regex crate.
    pub fn regex_pattern(&self) -> regex::Regex {
        regex::Regex::new(&self.get_regex_string())
            .expect("an emoji's regex always compiles")
    }
}
