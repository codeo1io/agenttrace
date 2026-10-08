# Generic-lane disclosure fixtures (run 5417681937ae assess PoCs)

Both files are byte-identical copies of the run-7c8a6446 assess PoC
corpus (`/tmp/at-assess-7c8a/corpus/`, swept between phases), checked in
per the fixture-durability contract so the goldens in
`tests/disclosure_plane_honesty.rs` never depend on /tmp state.

- `model-used-lowercase.jsonl` (sha256
  `1bf4c6717123302ec135997c904e41113c58ef7ded7727e66079923a24e2721c`)
  — the rm-718 PoC: multi-line role-only JSONL (no `type` keys) so the
  generic/hermes fallback lane parses it, with a lowercase
  `"model_used"` key and a canonical four-class usage block on the
  assistant row. At the pre-fix base the session attributed to
  `"default"` ($0.00, unpriced 1) because `Event.model_used`
  deserialized only PascalCase `ModelUsed`.
- `claude-flat-mixed.jsonl` — the assess skip-census PoC: flat
  `type:`-keyed lines plus a bare-string line (`"standalone note"`,
  `non_object_line`) and a torn final line (`unparseable_line`), pinning
  the rm-526 counted-loss channel the generic-adjacent lanes share.
- `cache-alias-camel.jsonl` — the rm-718 F4 arm: a generic-lane usage
  block whose cache-read count rides the Gemini wire spelling
  `cachedContentTokenCount`, which mapped nowhere at the pre-fix base
  (tokens_cache_r stayed 0).

Do not edit these files; they are golden inputs.
