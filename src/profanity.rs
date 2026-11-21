//! java: util/ProfanityFilter.java — the 1:1 logic (loadConfigs only runs
//! with the filter on; the upstream filterText call site is commented out
//! in the renderer, so nothing else calls it, exactly as shipped).

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;

/// java: smallWordsFile — the csv fetched from the swears repo.
const SMALL_WORDS_FILE: &str = "https://raw.githubusercontent.com/RobertJGabriel/Google-profanity-words/master/list.txt";

#[derive(Default)]
struct State {
    words: HashMap<String, Vec<String>>,
    largest_word_length: i32,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(State::default()))
}

/// java: ProfanityFilter.loadConfigs — the csv's lines, "word" or
/// "word,ignoredCombination", fill the map; largestWordLength tracks the
/// longest word seen.
pub fn load_configs() {
    let data = crate::common_class::read_string_from_url(SMALL_WORDS_FILE);
    let mut state = state().lock().unwrap();
    let mut largest = 0;
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut split = line.split(',');
        let word = split.next().unwrap_or_default().trim().to_string();
        let ignore_in_combination_with_words: Vec<String> = split
            .next()
            .map(|rest| {
                rest.split('|')
                    .map(|part| part.trim().to_lowercase())
                    .collect()
            })
            .unwrap_or_default();
        if word.chars().count() > largest as usize {
            largest = word.chars().count() as i32;
        }
        state
            .words
            .insert(word.to_lowercase(), ignore_in_combination_with_words);
    }
    state.largest_word_length = largest;
}

/// java: ProfanityFilter.badWordsFound(input) — the leet table first
/// normalizes the lowercase input, then every map word is searched whole;
/// an ignore list word found alongside does not count.
pub fn bad_words_found(input: &str) -> Vec<String> {
    let input = normalize_leet(input);
    let state = state().lock().unwrap();
    if state.largest_word_length == 0 {
        return Vec::new();
    }
    let mut found = Vec::new();
    let lower = input.to_lowercase();
    for (word, ignore_in_combination) in &state.words {
        let mut index = 0usize;
        while let Some(pos) = lower[index..].find(word) {
            let absolute = index + pos;
            let prior = if absolute == 0 {
                ""
            } else {
                &lower[absolute - 1..absolute]
            };
            let next = absolute + word.len();
            let after = if next >= lower.len() {
                ""
            } else {
                &lower[next..next + 1]
            };
            let continuation = if prior == "" && after == "" {
                true
            } else {
                !ignore_in_combination.is_empty()
                    && ignore_in_combination.contains(&prior.to_string())
            };
            if continuation {
                found.push(word.clone());
            }
            index = next;
            if index >= lower.len() {
                break;
            }
        }
    }
    found
}

/// java: filterText(string) — the filter's replacement runs through
/// regex over the word boundaries with the configured replacement string;
/// every found word becomes it.
pub fn filter_text(string: &str, config: &crate::Config) -> String {
    let mut out = string.to_string();
    for word in bad_words_found(string) {
        // java: string.replaceAll("\\b" + Pattern.quote(word) + "\\b",
        // replacement) — the boundary word becomes the filter string
        if let Ok(pattern) = regex::Regex::new(&format!(r"\b{}\b", regex::escape(&word))) {
            out = pattern
                .replace_all(&out, config.profanity_filter_replacement.as_str())
                .to_string();
        }
    }
    out
}

/// java: the leet table (a->@ 4, e->3, i->!, o->0, s->$ 5, t->7, g->9,
/// l->1) applied to the lowercase input before the search.
fn normalize_leet(input: &str) -> String {
    let lower = input.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    for c in lower.chars() {
        match c {
            '@' | '4' => out.push('a'),
            '3' => out.push('e'),
            '!' | '1' => out.push('i'),
            '0' => out.push('o'),
            '$' | '5' => out.push('s'),
            '7' => out.push('t'),
            '9' => out.push('g'),
            other => out.push(other),
        }
    }
    out
}
