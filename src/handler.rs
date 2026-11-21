//! java: ClientEmojiHandler.java — the load pipeline. initEmojis (the font
//! swap) rides the render seam: the registry work below is exact, the
//! renderer is only swapped in game where the mod API exposes a font hook.

use std::collections::BTreeMap;

use crate::category::EmojiCategory;
use crate::emoji::Emoji;
use crate::recipe::EmojiRecipe;
use crate::{common_class, Config, Registry};

/// java: setup() — preInitEmojis, (initEmojis is the font seam), indexEmojis,
/// then the "Loaded N emojis" info line.
pub fn setup(registry: &mut Registry, config: &Config) {
    pre_init_emojis(registry, config);
    index_emojis(registry);
    println!(
        "Emojiful: Loaded {} emojis",
        registry.emoji_list.len()
    );
}

/// java: indexEmojis() — ALL_EMOJIS is every emoji's strings flattened;
/// SORTED_EMOJIS_FOR_SELECTION chunks each category's emojis into rows of
/// nine (the leftover row carries the remainder) and lineAmount counts the
/// rows, one per category plus one per full row and one per leftover row.
pub fn index_emojis(registry: &mut Registry) {
    registry.all_emojis = registry
        .emoji_list
        .iter()
        .flat_map(|emoji| emoji.strings.iter().cloned())
        .collect();
    registry.sorted_selection.clear();
    registry.line_amount = 0;
    for category in &registry.categories.clone() {
        registry.line_amount += 1;
        let mut row: Vec<Emoji> = Vec::with_capacity(9);
        let mut has_row = false;
        let per_category = registry
            .emoji_map
            .get(&category.name)
            .cloned()
            .unwrap_or_default();
        for emoji in per_category {
            row.push(emoji);
            has_row = true;
            if row.len() >= 9 {
                registry
                    .sorted_selection
                    .entry(category.name.clone())
                    .or_default()
                    .push(std::mem::take(&mut row));
                registry.line_amount += 1;
            }
        }
        if has_row {
            registry
                .sorted_selection
                .entry(category.name.clone())
                .or_default()
                .push(row);
            registry.line_amount += 1;
        }
    }
}

/// java: preInitEmojis() — the eight standard categories, then custom
/// categories (front of the list), then Twemoji, then the profanity filter.
pub fn pre_init_emojis(registry: &mut Registry, config: &Config) {
    for name in [
        "Smileys & Emotion",
        "Animals & Nature",
        "Food & Drink",
        "Activities",
        "Travel & Places",
        "Objects",
        "Symbols",
        "Flags",
    ] {
        registry.categories.push(EmojiCategory::new(name, false));
    }
    if config.load_custom {
        load_custom_emojis(registry);
    }
    // java: //loadGithubEmojis(); — commented out upstream, kept commented
    if config.load_twemoji {
        load_twemojis(registry);
    }
    if config.profanity_filter {
        super::profanity::load_configs();
    }
}

/// java: loadCustomEmojis() — the Categories.yml list from the
/// emojiful-assets master branch; every listed file loads and its
/// category (file name minus .yml) enters the category list at index 0
/// with its emojis replacing the map entry. An exception sets the error
/// flag and logs.
pub fn load_custom_emojis(registry: &mut Registry) {
    let categories_raw = match common_class::read_string_from_url(
        "https://raw.githubusercontent.com/InnovativeOnlineIndustries/emojiful-assets/master/Categories.yml",
    )
    .pipe(|raw| serde_yaml::from_str::<Vec<String>>(&raw))
    {
        Ok(categories) => categories,
        Err(error) => {
            registry.error = true;
            eprintln!("emojiful: An exception was caught whilst loading custom emojis: {}", error);
            return;
        }
    };
    for category in categories_raw {
        registry.categories.insert(
            0,
            EmojiCategory::new(&category.replace(".yml", ""), false),
        );
        let raw = match common_class::read_category_raw(&category) {
            Ok(raw) => raw,
            Err(error) => {
                registry.error = true;
                eprintln!("emojiful: An exception was caught whilst loading custom emojis: {}", error);
                return;
            }
        };
        let emojis = match parse_category_yaml(&raw) {
            Ok(emojis) => emojis,
            Err(error) => {
                registry.error = true;
                eprintln!("emojiful: An exception was caught whilst loading custom emojis: {}", error);
                return;
            }
        };
        let name = category.replace(".yml", "");
        registry.emoji_list.extend(emojis.iter().cloned());
        registry.emoji_map.insert(name, emojis);
    }
}

/// java: readCategory parses through yamlbeans into Emoji[].class — the
/// YAML list of emoji objects with the class's field names.
pub fn parse_category_yaml(raw: &str) -> Result<Vec<Emoji>, String> {
    let parsed: Vec<serde_yaml::Value> =
        serde_yaml::from_str(raw).map_err(|error| error.to_string())?;
    let mut emojis = Vec::new();
    for element in parsed {
        let field = |key: &str| -> Option<String> {
            element
                .get(key)
                .and_then(|value| value.as_str().map(str::to_string))
        };
        let strings_field = || -> Vec<String> {
            element
                .get("strings")
                .and_then(|value| value.as_sequence())
                .map(|sequence| {
                    sequence
                        .iter()
                        .filter_map(|value| value.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default()
        };
        emojis.push(Emoji {
            name: field("name").unwrap_or_default(),
            strings: strings_field(),
            texts: element
                .get("texts")
                .and_then(|value| value.as_sequence())
                .map(|sequence| {
                    sequence
                        .iter()
                        .filter_map(|value| value.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            location: field("location").unwrap_or_default(),
            version: element
                .get("version")
                .and_then(|value| value.as_i64())
                .unwrap_or(1) as i32,
            sort: element
                .get("sort")
                .and_then(|value| value.as_i64())
                .unwrap_or(0) as i32,
            world_based: false,
        });
    }
    Ok(emojis)
}

/// java: loadTwemojis() — iamcal/emoji-data's emoji.json, every entry with
/// has_img_twitter becomes an EmojiFromTwitmoji (name = short_name,
/// location = image, sort = sort_order, one string per short_name wrapped
/// in colons, :swear: added on face_with_symbols_on_mouth, texts copied
/// unless null), keyed by category. EMOJI_WITH_TEXTS sorts by sort and so
/// does every category's list. An exception sets the error flag and logs.
pub fn load_twemojis(registry: &mut Registry) {
    let json = match common_class::read_json_from_url(
        "https://raw.githubusercontent.com/iamcal/emoji-data/master/emoji.json",
    ) {
        Ok(json) => json,
        Err(error) => {
            registry.error = true;
            eprintln!("emojiful: Emojiful found an error while loading: {}", error);
            return;
        }
    };
    let Some(elements) = json.as_array() else {
        registry.error = true;
        eprintln!("emojiful: Emojiful found an error while loading: not an array");
        return;
    };
    for element in elements {
        let object = element.as_object().unwrap();
        let has_twitter = object
            .get("has_img_twitter")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        if !has_twitter {
            continue;
        }
        let name = object
            .get("short_name")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        let location = object
            .get("image")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        let sort = object
            .get("sort_order")
            .and_then(|value| value.as_i64())
            .unwrap_or(0) as i32;
        let mut strings: Vec<String> = object
            .get("short_names")
            .and_then(|value| value.as_array())
            .map(|array| {
                array
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_string))
                    .map(|short| format!(":{}:", short))
                    .collect()
            })
            .unwrap_or_default();
        if strings.iter().any(|string| string == ":face_with_symbols_on_mouth:") {
            strings.push(":swear:".to_string());
        }
        let texts: Vec<String> = object
            .get("texts")
            .filter(|value| !value.is_null())
            .and_then(|value| value.as_array())
            .map(|array| {
                array
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let category_name = object
            .get("category")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        let emoji = Emoji {
            name,
            strings,
            texts,
            location,
            version: 1,
            sort,
            world_based: false,
        };
        let has_texts = !emoji.texts.is_empty();
        registry
            .emoji_map
            .entry(category_name)
            .or_default()
            .push(emoji.clone());
        registry.emoji_list.push(emoji.clone());
        if has_texts {
            registry.emoji_with_texts.push(emoji);
        }
    }
    registry
        .emoji_with_texts
        .sort_by_key(|emoji| emoji.sort);
    for list in registry.emoji_map.values_mut() {
        list.sort_by_key(|emoji| emoji.sort);
    }
}

/// java: ForgeClientHandler.onRecipesUpdated — every world-based category
/// and emoji leaves the registry first; then, with loadDatapack on, each
/// EmojiRecipe (category, name, url) becomes an EmojiFromGithub
/// (worldBased, one string :name:, location = name, url carried), its
/// category enters the category list at index 0 when absent (case
/// differs), and indexEmojis runs again. The recipe-manager event rides
/// the mod API seam; the port runs the same registry work.
pub fn on_recipes_updated(registry: &mut Registry, config: &Config, recipes: &[EmojiRecipe]) {
    registry.categories.retain(|category| !category.world_based);
    registry.emoji_list.retain(|emoji| !emoji.world_based);
    if config.load_datapack {
        for recipe in recipes {
            let emoji = Emoji {
                name: recipe.name.clone(),
                strings: vec![format!(":{}:", recipe.name)],
                texts: Vec::new(),
                location: recipe.name.clone(),
                version: 1,
                sort: 0,
                world_based: true,
            };
            let category = recipe.category.clone();
            if !registry
                .categories
                .iter()
                .any(|existing| existing.name.eq_ignore_ascii_case(&category))
            {
                registry
                    .categories
                    .insert(0, EmojiCategory::new(&category, true));
            }
            registry.emoji_list.push(emoji.clone());
            registry
                .emoji_map
                .entry(category)
                .or_default()
                .push(emoji);
        }
        index_emojis(registry);
    }
}

/// Small pipe helper mirroring the try-chain style of the originals.
trait Pipe: Sized {
    fn pipe<R, F: FnOnce(Self) -> R>(self, f: F) -> R {
        f(self)
    }
}
impl<T> Pipe for T {}
