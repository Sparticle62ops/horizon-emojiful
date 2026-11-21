//! Exact 1:1 Rust translation of Emojiful by Innovative Online Industries
//! (github.com/InnovativeOnlineIndustries/Emojiful, MIT; see LICENSE.md).
//! The registry and pipeline are `handler` <-> ClientEmojiHandler.java
//! (with preInitEmojis, loadCustomEmojis, loadTwemojis, indexEmojis); the
//! emoji model and regex builders are `emoji` <-> api/Emoji.java; the
//! category record is `category` <-> api/EmojiCategory.java; the font
//! renderer's scan is `emoji_util` <-> EmojiFontRenderer.getEmojiFormattedString
//! with EmojiUtil.cleanStringForRegex; the URL readers are
//! `common_class` <-> CommonClass.java; the filter is `profanity` <->
//! util/ProfanityFilter.java; the datapack recipes are `recipe` <->
//! datapack/EmojiRecipe(.Serializer).java; the config is `config` here <>
//! platform/FabricConfigHelper.java.
//!
//! Wired on the mod API: the nine config toggles register as settings
//! (same defaults), the config file config/emojiful.json carries the same
//! camelCase fields as MidnightConfig (the profanityFilterReplacement
//! string stays file-only; the settings layer has toggles only), and the
//! client_setup hook runs the original's setup: the load pipeline, then
//! the index. The onRecipesUpdated registry work (world-based cleanup,
//! datapack emojis, re-index) is ported and runs when recipes arrive.
//!
//! Seams (upstream behaviors the mod API does not expose yet, tracked):
//! - initEmojis swaps the game's font renderer for EmojiFontRenderer and
//!   registers the sign renderer; the mod API has no font-renderer or
//!   block-entity hook, so the scan logic lives in emoji_util and the
//!   textures stay undrawn until the hook exists. As packaged the mod has
//!   no visible effect in game, exactly what the review noted upstream.
//! - the emoji selector, autocomplete and config screens are unported;
//!   the selection data (SORTED_EMOJIS_FOR_SELECTION) is built.
//! - GIF splitting, texture uploads and the image cache are render-side
//!   and follow the same seam; the locations and versions still load.
//! - the recipe-manager event has no mod API hook; on_recipes_updated
//!   holds the registry work for when one lands.
//!
//! Licence: MIT, carried from the original (see LICENSE.md).

pub mod category;
pub mod common_class;
pub mod emoji;
pub mod emoji_util;
pub mod handler;
pub mod profanity;
pub mod recipe;
#[cfg(test)]
pub mod tests;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;

use mod_api::{ClientHost, ClientSetupEvent, Registrar, RegistrationContext, SettingSpec};

pub use crate::category::EmojiCategory;
pub use crate::emoji::Emoji;
pub use crate::recipe::EmojiRecipe;

/// java: EmojiType's CONFIG pattern — the original's MidnightConfig file
/// is config/emojiful.json with the FabricConfigHelper fields.
pub fn game_dir() -> PathBuf {
    std::env::var("XI_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn config_path() -> PathBuf {
    game_dir().join("config").join("emojiful.json")
}

/// java: FabricConfigHelper's @Entry fields, same defaults. The field
/// names are the original's camelCase in the file.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(rename = "renderEmoji")]
    pub render_emoji: bool,
    #[serde(rename = "emojiSelector")]
    pub emoji_selector: bool,
    #[serde(rename = "emojiAutocomplete")]
    pub emoji_autocomplete: bool,
    #[serde(rename = "gifEmojis")]
    pub gif_emojis: bool,
    #[serde(rename = "emojiReplacement")]
    pub emoji_replacement: bool,
    #[serde(rename = "loadTwemoji")]
    pub load_twemoji: bool,
    #[serde(rename = "loadCustom")]
    pub load_custom: bool,
    #[serde(rename = "loadDatapack")]
    pub load_datapack: bool,
    #[serde(rename = "profanityFilter")]
    pub profanity_filter: bool,
    #[serde(rename = "profanityFilterReplacement")]
    pub profanity_filter_replacement: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            render_emoji: true,
            emoji_selector: true,
            emoji_autocomplete: true,
            gif_emojis: true,
            emoji_replacement: true,
            load_twemoji: true,
            load_custom: true,
            load_datapack: true,
            profanity_filter: false,
            profanity_filter_replacement: ":swear:".to_string(),
        }
    }
}

/// java: ConfigManager's load/save pattern — a config that fails to parse
/// becomes the default, and saving writes the file when it can.
pub fn load_config(path: &PathBuf) -> Config {
    match std::fs::read_to_string(path) {
        Ok(json) => serde_json::from_str::<Config>(&json).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn save_config(path: &PathBuf, config: &Config) -> bool {
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    match serde_json::to_string_pretty(config) {
        Ok(json) => std::fs::write(path, json).is_ok(),
        Err(_) => false,
    }
}

/// java: Constants' loaded state — CATEGORIES (an insertion-ordered list),
/// EMOJI_MAP (insertion-ordered by category), EMOJI_LIST, ALL_EMOJIS,
/// SORTED_EMOJIS_FOR_SELECTION, EMOJI_WITH_TEXTS, lineAmount, and the
/// ERROR flag. The game drives these hooks from one thread, like the
/// original's static fields.
#[derive(Default)]
pub struct Registry {
    pub categories: Vec<EmojiCategory>,
    pub emoji_map: BTreeMap<String, Vec<Emoji>>,
    pub emoji_list: Vec<Emoji>,
    pub all_emojis: Vec<String>,
    pub sorted_selection: BTreeMap<String, Vec<Vec<Emoji>>>,
    pub emoji_with_texts: Vec<Emoji>,
    pub line_amount: i32,
    /// java: Constants.ERROR
    pub error: bool,
}

thread_local! {
    static STATE: RefCell<Registry> = RefCell::new(Registry::default());
}

pub fn with_registry<R>(f: impl FnOnce(&mut Registry) -> R) -> R {
    STATE.with(|cell| f(&mut cell.borrow_mut()))
}

fn set_state(registry: Registry) {
    STATE.with(|cell| *cell.borrow_mut() = registry);
}

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {
    /// java: FabricConfigHelper's @Entry booleans as settings, same
    /// defaults and keys; the string stays in the config file.
    fn register(&self, registrar: &mut dyn Registrar, _context: &RegistrationContext) {
        let _ = registrar.register_setting(SettingSpec::toggle(
            "render_emoji",
            "setting.emojiful.renderEmoji",
            true,
        ));
        let _ = registrar.register_setting(SettingSpec::toggle(
            "emoji_selector",
            "setting.emojiful.emojiSelector",
            true,
        ));
        let _ = registrar.register_setting(SettingSpec::toggle(
            "emoji_autocomplete",
            "setting.emojiful.emojiAutocomplete",
            true,
        ));
        let _ = registrar.register_setting(SettingSpec::toggle(
            "gif_emojis",
            "setting.emojiful.gifEmojis",
            true,
        ));
        let _ = registrar.register_setting(SettingSpec::toggle(
            "emoji_replacement",
            "setting.emojiful.emojiReplacement",
            true,
        ));
        let _ = registrar.register_setting(SettingSpec::toggle(
            "load_twemoji",
            "setting.emojiful.loadTwemoji",
            true,
        ));
        let _ = registrar.register_setting(SettingSpec::toggle(
            "load_custom",
            "setting.emojiful.loadCustom",
            true,
        ));
        let _ = registrar.register_setting(SettingSpec::toggle(
            "load_datapack",
            "setting.emojiful.loadDatapack",
            true,
        ));
        let _ = registrar.register_setting(SettingSpec::toggle(
            "profanity_filter",
            "setting.emojiful.profanityFilter",
            false,
        ));
    }

    /// java: ClientEmojiHandler.setup() — the config loads (settings
    /// overlay it, like the original's TOML overlay), then the load
    /// pipeline and the index run.
    fn client_setup(&self, host: &mut dyn ClientHost, _event: &mut ClientSetupEvent) {
        let path = config_path();
        let mut config = load_config(&path);
        for (key, field) in [
            ("render_emoji", 0usize),
            ("emoji_selector", 1),
            ("emoji_autocomplete", 2),
            ("gif_emojis", 3),
            ("emoji_replacement", 4),
            ("load_twemoji", 5),
            ("load_custom", 6),
            ("load_datapack", 7),
            ("profanity_filter", 8),
        ] {
            if let Some(value) = host.setting_bool(key) {
                match field {
                    0 => config.render_emoji = value,
                    1 => config.emoji_selector = value,
                    2 => config.emoji_autocomplete = value,
                    3 => config.gif_emojis = value,
                    4 => config.emoji_replacement = value,
                    5 => config.load_twemoji = value,
                    6 => config.load_custom = value,
                    7 => config.load_datapack = value,
                    _ => config.profanity_filter = value,
                }
            }
        }
        save_config(&path, &config);
        let mut registry = Registry::default();
        handler::setup(&mut registry, &config);
        set_state(registry);
    }
}
