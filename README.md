# horizon-emojiful

Emojiful ported to Horizon as a native Rust mod (slice 1).

- Upstream: https://github.com/InnovativeOnlineIndustries/Emojiful at commit 6442d9d (2024-06-12)
- License: MIT (c) 2022 Innovative Online Industries, carried verbatim in LICENSE.md
- Port policy: 1:1 feature port; no optimizations or behavior changes

## Slice 1 (ported, 26/26 tests)

- Config model (MidnightConfig field names as camelCase serde keys, upstream defaults, broken file falls back to defaults)
- Emoji model: name/strings/texts/location/version/sort/worldBased, case-insensitive binding test, shortest-string pick, regex builder (letter-boundary logic + cleanStringForRegex escaping `) ( | *`), text-emoticon regex with whitespace lookarounds (pattern string kept verbatim), asset URL (emojiful-assets base), cache path `emojiful/cache/<name>-<version>`, texture-id name sanitizer
- EmojiFromGithub (own `url` override); EmojiFromTwitmoji / EmojiFromEmojipedia share the default URL upstream and only differ in texture upload (seam)
- Emoji quad geometry from EmojiUtil.renderEmoji (size 10, offset (0,1), texture 16) and the GIF frame-strip model (delay repeats per frame)
- ProfanityFilter: CSV load (word + ignore-in-combination list, Java split trailing-empty semantics), substring scan with leet normalization (1!→i 3→e 4a @a 5s 7t 0o 9g), replacement pass
- ClientEmojiHandler: 8 vanilla categories, custom-package loading (Categories.yml + per-category YAML), twemoji loading from iamcal/emoji-data (has_img_twitter filter, colon-wrapped short_names, :swear: alias, texts, sort_order ordering), emoji indexing into 9-slot selection rows with upstream lineAmount counting, datapack emoji flow (world-based GitHub emojis + category insert + re-index), shouldKeyBeIgnored
- Hand-rolled YAML readers for the two real emojiful-assets formats (no new dependencies beyond serde/serde_json)

## Game-side seams (degradation notes)

1. Font renderer swap + emoji glyph rendering (EmojiFontRenderer, EmojiUtil.createRenderType/renderEmoji): Horizon needs a chat-font hook and a texture-quad glyph path.
2. Texture upload + GIF animation (Emoji.DownloadImageData, Twitmoji/Emojipedia custom image data): needs a dynamic-texture registration surface.
3. Asset fetch (CommonClass.readStringFromURL, Emoji.loadTextureFromServer, ProfanityFilter CSV fetch): needs an async HTTP surface for native mods.
4. Datapack recipe lookup (RecipeManager.getAllRecipesFor for the emojiful recipe type): needs a custom-recipe surface.
5. Chat GUI: autocomplete suggestions (EmojiSuggestionHelper), emoji selector screen, keybind handling in the chat screen.

Per standing policy these map to issues/PRs on horizon-rust/horizon as each surface is needed.
