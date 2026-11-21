//! java: CommonClass.java — the URL readers.

/// java: readStringFromURL — the whole body as UTF-8 text; an IOException
/// prints and returns "".
pub fn read_string_from_url(request_url: &str) -> String {
    match ureq::get(request_url).call() {
        Ok(response) => response.into_string().unwrap_or_default(),
        Err(error) => {
            eprintln!("emojiful: {}", error);
            String::new()
        }
    }
}

/// java: readJsonFromUrl — parse the fetched body as JSON.
pub fn read_json_from_url(url: &str) -> Result<serde_json::Value, String> {
    let json_text = read_string_from_url(url);
    serde_json::from_str(&json_text).map_err(|error| error.to_string())
}

/// java: readCategory(String) — the category's YAML from the
/// emojiful-assets master branch, parsed as an Emoji array. (The original
/// parses through yamlbeans into Emoji[].class; this reads the same files
/// with the same field names.)
pub fn read_category_raw(category: &str) -> Result<String, String> {
    let url = format!(
        "https://raw.githubusercontent.com/InnovativeOnlineIndustries/emojiful-assets/master/{}",
        category
    );
    Ok(read_string_from_url(&url))
}
