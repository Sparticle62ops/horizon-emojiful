# horizon-emojiful

Emojiful ported to Horizon as a native Rust mod. Registry id: `emojiful_port`
(the upstream `emojiful` id is a live registry entry owned by another
publisher, so this port lists under a distinct id; the runtime id scopes the
settings the same way).

- Upstream: https://github.com/InnovativeOnlineIndustries/Emojiful at commit 6442d9d (2024-06-12)
- License: MIT (c) 2022 Innovative Online Industries, carried verbatim in LICENSE.md
- Port policy: exact 1:1 Rust translation; no optimizations or behavior changes

## What is ported (12 tests)

- Config model: the original's nine toggles register as settings with the
  upstream lang labels and defaults; `config/emojiful.json` keeps the
  camelCase fields (the replacement string stays file-only) and a broken file
  falls back to defaults, like MidnightConfig.
- Emoji model (`api/Emoji.java`): name/strings/texts/location/version/sort/
  worldBased, case-insensitive binding test, shortest-string pick, the regex
  builder (letter-boundary logic, escaping applied after the `\b` edges) and
  the text-emoticon regex with whitespace lookarounds (pattern kept verbatim;
  it rides the render seam).
- `EmojiFontRenderer.getEmojiFormattedString`'s scan: formatting-stripped
  copy scanned case-sensitively, each hit mapped at the character index of
  its code in the already-replaced original (an indexOf miss maps -1, the
  original's quirk), later indexes shifted down by the replaced length minus
  the one-character snowman, and both copies replace the hit with U+2603.
- `EmojiUtil.cleanStringForRegex` (escapes exactly `) ( | *`).
- `ClientEmojiHandler`: 8 vanilla categories, custom-package loading
  (Categories.yml + per-category YAML from emojiful-assets), Twemoji loading
  from iamcal/emoji-data (has_img_twitter filter, colon-wrapped short_names,
  the :swear: alias, texts, sort ordering), indexing into 9-slot selection
  rows with the original's lineAmount counting, and the datapack flow
  (`onRecipesUpdated`: world-based cleanup, `:name:` strings, category
  entered with worldBased = true, re-index).
- ProfanityFilter: CSV load, substring scan with leet normalization, the
  replacement pass (the filterText call site stays commented out upstream, so
  nothing else calls it, exactly as shipped).

## Game-side seams (degradation notes)

1. Font renderer swap + emoji glyph rendering + texture/GIF upload
   (EmojiFontRenderer, EmojiUtil, Emoji DownloadImageData): the mod API has no
   font-renderer or dynamic-texture hook, so as packaged the mod has no
   visible effect in game, exactly what the upstream review noted.
2. Chat GUI: autocomplete suggestions (EmojiSuggestionHelper), emoji selector
   screen, keybind handling in the chat screen are unported; the selection
   data (SORTED_EMOJIS_FOR_SELECTION) is built.
3. Datapack recipe lookup (RecipeManager.getAllRecipesFor for the emojiful
   recipe type) has no mod API hook; onRecipesUpdated holds the registry work.

Per standing policy these map to issues/PRs on horizon-rust/horizon as each
surface is needed.
