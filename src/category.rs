//! java: api/EmojiCategory.java.

/// java: record EmojiCategory(String name, boolean worldBased).
#[derive(Clone, Debug, PartialEq)]
pub struct EmojiCategory {
    pub name: String,
    pub world_based: bool,
}

impl EmojiCategory {
    pub fn new(name: &str, world_based: bool) -> Self {
        EmojiCategory {
            name: name.to_string(),
            world_based,
        }
    }
}
