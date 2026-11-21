//! Tests standing in for the original's behaviors: the regex builders,
//! the font-renderer scan, the category YAML parsing, the twemoji JSON
//! field mapping, the index chunking, the profanity filter and the recipe
//! parse. All fixtures, no network.

use crate::emoji::Emoji;
use crate::emoji_util::{clean_string_for_regex, get_emoji_formatted_string, strip_formatting};
use crate::handler::{index_emojis, on_recipes_updated, parse_category_yaml};
use crate::recipe::EmojiRecipe;
use crate::{Config, Registry};

fn emoji(name: &str, strings: &[&str]) -> Emoji {
    Emoji::new(
        name.to_string(),
        strings.iter().map(|s| s.to_string()).collect(),
        Vec::new(),
        format!("assets/{}.png", name),
    )
}

#[test]
fn clean_string_for_regex_escapes_exactly_the_four_chars() {
    assert_eq!(clean_string_for_regex("a)b(c|d*e"), r"a\)b\(c\|d\*e");
    assert_eq!(clean_string_for_regex(":smiley:"), ":smiley:");
}

#[test]
fn regex_string_wraps_letter_edges_and_joins_with_pipe() {
    let e = emoji("smiley", &[":)", ":smiley:"]);
    // ':' is not a letter, so neither ':)' nor ':smiley:' gains \b
    assert_eq!(e.get_regex_string(), r":\)|:smiley:");
    let e2 = emoji("kiss", &[":kiss:", "xkissx"]);
    // ':kiss:' starts and ends with ':' -> no \b; 'xkissx' starts and ends
    // with a letter -> \b on both edges, added before the escaping pass
    assert_eq!(e2.get_regex_string(), r":kiss:|\bxkissx\b");
}

#[test]
fn text_regex_keeps_the_lookarounds_verbatim() {
    let mut e = emoji("heart", &[":heart:"]);
    e.texts = vec!["love".to_string(), "<3".to_string()];
    assert_eq!(e.get_text_regex(), r"(?<=^|\s)(love|<3)(?=$|\s)");
}

#[test]
fn test_matches_ignoring_case_only() {
    let e = emoji("smiley", &[":)", ":smiley:"]);
    assert!(e.test(":SMILEY:"));
    assert!(e.test(":smiley:"));
    assert!(!e.test(":sad:"));
}

#[test]
fn strip_formatting_removes_allowed_codes_only() {
    assert_eq!(strip_formatting("\u{00a7}ahello"), "hello");
    assert_eq!(strip_formatting("\u{00a7}zstay"), "stay");
    assert_eq!(strip_formatting("a\u{00a7}lb"), "ab");
}

#[test]
fn scan_maps_and_shifts_indices_like_the_renderer() {
    let config = Config::default();
    let list = vec![emoji("smiley", &[":smiley:"]), emoji("kiss", &[":kiss:"])];
    let (text, map) = get_emoji_formatted_string(":smiley: hi :kiss:", &config, &list);
    // each hit maps the character index of its code in the already-replaced
    // text: smiley at 0, ':kiss:' at 5 after the first snowman
    let keys = map.keys();
    assert_eq!(text, "☃ hi ☃");
    assert!(keys.contains(&0), "smiley at 0");
    assert!(keys.contains(&5), "kiss at 5 in the replaced text");
    // the index order in the map is insertion order: smiley first
    assert_eq!(keys[0], 0);
}

#[test]
fn scan_respects_the_render_flag_and_formatting() {
    let mut config = Config::default();
    config.render_emoji = false;
    let list = vec![emoji("smiley", &[":smiley:"])];
    let (text, map) = get_emoji_formatted_string(":smiley:", &config, &list);
    assert_eq!(text, ":smiley:");
    assert!(map.keys().is_empty(), "nothing mapped with render off");
    let config = Config::default();
    // a formatting code inside the code: the stripped copy matches, the
    // original indexOf misses (-1), exactly the original's quirk
    let (text2, map) = get_emoji_formatted_string(":s\u{00a7}7miley:", &config, &list);
    assert_eq!(text2, ":s\u{00a7}7miley:");
    assert_eq!(map.keys(), vec![-1], "the miss maps -1 like the original's indexOf");
}

#[test]
fn category_yaml_parses_with_the_class_fields() {
    let raw = "- name: smiley\n  strings:\n    - \":)\"\n    - \":smiley:\"\n  location: \"assets/faces/smiley.png\"\n- name: thinking\n  strings:\n    - \":thinking:\"\n  location: \"assets/faces/thinking.png\"\n";
    let emojis = parse_category_yaml(raw).unwrap();
    assert_eq!(emojis.len(), 2);
    assert_eq!(emojis[0].name, "smiley");
    assert_eq!(emojis[0].strings, vec![":)", ":smiley:"]);
    assert_eq!(emojis[0].location, "assets/faces/smiley.png");
    assert_eq!(emojis[0].version, 1);
    assert_eq!(emojis[0].sort, 0);
    assert!(!emojis[0].world_based);
}

#[test]
fn index_emojis_chunks_rows_of_nine_and_counts_lines() {
    let mut registry = Registry::default();
    registry.categories.push(crate::EmojiCategory::new("Test", false));
    let mut emojis = Vec::new();
    for i in 0..10 {
        emojis.push(emoji(&format!("e{}", i), &[&format!(":e{}:", i)]));
    }
    registry.emoji_list = emojis.clone();
    registry.emoji_map.insert("Test".to_string(), emojis);
    index_emojis(&mut registry);
    let rows = &registry.sorted_selection["Test"];
    assert_eq!(rows.len(), 2, "one full row of nine plus the remainder");
    assert_eq!(rows[0].len(), 9);
    assert_eq!(rows[1].len(), 1);
    assert_eq!(registry.line_amount, 3, "one per category + full row + leftover");
    assert_eq!(registry.all_emojis.len(), 10);
}

#[test]
fn recipes_rebuild_world_emojis_and_reindex() {
    let mut registry = Registry::default();
    registry
        .emoji_map
        .insert("Custom".to_string(), vec![{
            let mut e = emoji("world", &[":world:"]);
            e.world_based = true;
            e
        }]);
    registry.categories.push(crate::EmojiCategory {
        name: "Custom".to_string(),
        world_based: true,
    });
    registry.categories.push(crate::EmojiCategory::new("Vanilla", false));
    let config = Config::default();
    let recipes = vec![EmojiRecipe {
        recipe_name: "emojiful:crown".to_string(),
        category: "custom".to_string(),
        name: "crown".to_string(),
        url: "https://example.com/crown.png".to_string(),
    }];
    on_recipes_updated(&mut registry, &config, &recipes);
    // java: the pre-existing world category is dropped, but the recipe
    // category enters with worldBased = true and leaves at the NEXT
    // recipes update, so it is still flagged here
    let world_flags: Vec<bool> = registry.categories.iter().map(|category| category.world_based).collect();
    assert_eq!(world_flags, vec![true, false], "recipe category world-based until the next pass");
    assert_eq!(registry.emoji_list.len(), 1, "only the datapack emoji stays");
    assert_eq!(registry.emoji_list[0].strings, vec![":crown:"]);
    assert!(registry.emoji_list[0].world_based);
    assert_eq!(registry.categories[0].name, "custom", "the recipe category enters at 0");
    // java: the map is not cleaned and getOrDefault gives Vanilla nothing,
    // so only the recipe category lands in the index
    assert_eq!(registry.sorted_selection.len(), 1, "reindexed");
}

#[test]
fn recipe_json_reads_the_serializer_fields() {
    let json = serde_json::json!({
        "category": "Faces",
        "name": "crown",
        "url": "https://example.com/crown.png"
    });
    let recipe = EmojiRecipe::from_json("emojiful:crown", &json);
    assert_eq!(recipe.recipe_name, "emojiful:crown");
    assert_eq!(recipe.category, "Faces");
    assert_eq!(recipe.name, "crown");
    assert_eq!(recipe.url, "https://example.com/crown.png");
}

#[test]
fn config_defaults_match_the_original_entries() {
    let config = Config::default();
    assert!(config.render_emoji);
    assert!(config.emoji_selector);
    assert!(config.emoji_autocomplete);
    assert!(config.gif_emojis);
    assert!(config.emoji_replacement);
    assert!(config.load_twemoji);
    assert!(config.load_custom);
    assert!(config.load_datapack);
    assert!(!config.profanity_filter);
    assert_eq!(config.profanity_filter_replacement, ":swear:");
    // the file keeps the original's camelCase names
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("\"renderEmoji\""));
    assert!(json.contains("\"profanityFilterReplacement\""));
}
