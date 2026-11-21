//! java: datapack/EmojiRecipe.java and EmojiRecipeSerializer.fromJson —
//! the datapack recipe (category, name, url, id) and its JSON parsing.

#[derive(Clone, Debug)]
pub struct EmojiRecipe {
    /// java: recipeName, the ResourceLocation id
    pub recipe_name: String,
    pub category: String,
    pub name: String,
    pub url: String,
}

impl EmojiRecipe {
    /// java: EmojiRecipeSerializer.fromJson — the json object's fields
    /// read with getString on category, name, url and the id under "id"
    /// (the serializer's key), falling back to nothing when absent.
    pub fn from_json(id: &str, json: &serde_json::Value) -> EmojiRecipe {
        let field = |key: &str| {
            json.get(key)
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string()
        };
        EmojiRecipe {
            recipe_name: id.to_string(),
            category: field("category"),
            name: field("name"),
            url: field("url"),
        }
    }
}
