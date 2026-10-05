# Prevention rule: string-anchored edits need unique-anchor and block-bound verification

- **Class:** workflow / manual-and-tool file edits (roadmap rows, code blocks, patch prep)
- **Observed:** 2026-10-05, run `32192d92` (repository-maintenance `d3046844` cycle 1) — hit twice in one cycle by two different tools: roadmap phase `ad571cc0` and implement phase `90c377d4`
- **Cost:** (1) a rider note landed on 3 wrong roadmap rows before its own verification grep caught it — redo plus a re-audit of every row the bad append touched; (2) a full code block (the post-loop copilot credit emission) silently failed to land, and the suite still passed — only a live probe returning `cost 0.0` exposed it, one turn later than the edit that "succeeded".

## What happened

Both failures share one shape: an edit anchored on a string that the author had not verified to be unique.

1. **Roadmap append (ad571cc0).** A rider note was appended after a sentence taken from a prior cycle's record. That sentence ("pairing this item with …" family) turned out to be cross-campaign boilerplate shared by **4** roadmap rows; the append landed on all of them, 3 of them wrongly. The phase's post-write grep caught it; the fix re-anchored on rm-042's own unique text and added a regex block-boundary check (the match must sit between the target row's header and the next row boundary) before writing.
2. **Code edit (90c377d4).** An `edit` call anchored on an `oldText` region that appeared in two places: the tool replaced one occurrence and the intended second block (the post-loop credit emission) was silently dropped. `cargo test` stayed green — the emission is additive behavior with no existing coverage — and the delta looked complete in the numstat. A live PoC replay (`--sessions` still reporting `cost 0.0` where $10.72 was expected) exposed the missing block.

The dangerous property in both cases: **the failure is silent**. No tool errors, gates stay green, and the defect surfaces only in downstream evidence — or never.

## Prevention rules

1. **Verify the anchor before writing, not after.** For any append/replace keyed by a content string, run the uniqueness probe first: the anchor must match **exactly once** in the target file (`grep -cF '<anchor>' file` == 1, or the editor's own search). If it matches twice, extend the anchor with adjacent unique context — do not hope.
2. **Verify the anchor is inside the intended block.** Uniqueness in the whole file is not enough when the file has repeated structure (roadmap rows, sibling functions, repeated literals). Bound the search: between the row's id/header line and the next row boundary, or inside the enclosing function. A match outside those bounds is the wrong match.
3. **Re-verify after writing.** Grep the inserted content: it must appear exactly once, in the intended block. For appends near repeated structures, also assert the block-boundary again after the edit.
4. **Treat a green suite as insufficient for silent-misapply classes.** An edit whose subject is additive behavior (new emission, new field, new path) needs a behavioral probe of that subject — a live replay of the motivating PoC, or a test that pins the new value — before the phase claims the edit landed. Numstat alone proves nothing.
5. **Prefer structured anchors over prose.** Row ids (`- id: \`rm-485\``), function signatures, or generated markers beat prose sentences, because their uniqueness is checkable — and their duplicates are greppable. Prose copied from another cycle's record is the classic wrong anchor.

## Related

- Fleet rule lineage: PR-J (evidence must be mirrored at the producing phase) covers the *evidence* side of the same cycle; this rule covers the *edit* side.
- Detection corroboration: the implement-phase live probe that caught case (2) is the same red-first PoC discipline this cycle's batch was selected on — see `docs/stewardship/2026-10-05-cycle1-compound-record-run32192d92.md`.
