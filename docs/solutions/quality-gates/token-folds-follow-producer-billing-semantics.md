# Token folds follow producer billing semantics

- Minted: 2026-10-11, run 5a04ae3b0b9f (repository-maintenance 62052d7689d04bafab23fbef59139aea cycle 3, compound 396cfce2), from the rm-619 implementation (implement e0a29b1b; research oracle 15c8c46d; roadmap FIX CONVENTION rider ae668616). Corrected at review (independent review 04eedd96 F1/F3, fix attempt f89a289b): the first cut of this very doc described the convention as "stored output INCLUSIVE of reasoning" — a replacement-style reading whose code dropped a mixed message's own `output`; both oracles are ADDITIVE.
- Applies to: any parser change that folds a usage sub-field (reasoning, cache, audio, cached-thinking, …) into an accounted token bucket.

## Rule

Before folding a usage sub-field into an accounted bucket, determine how the PRODUCER persists and BILLS it, and fold accordingly. The question is not "which bucket is most convenient" but "does the producer's stored `output` already include this quantity, and does the provider bill it at the output rate?"

- **Persisted sibling with cost → ADD onto output, keep the breakdown visible.** opencode persists `tokens.reasoning` as a first-class sibling of input/output (session/message.ts tokens schema; the ai-sdk adapter maps `reasoningTokens ?? outputTokenDetails.reasoningTokens` into it), and the opencode_db convention is ADDITIVE — `tokens_output.saturating_add(stored_reasoning.max(0))` at sqlite_sessions.rs:522/:876, the same additive shape as qwen CU-20 (`let output = output.saturating_add(reasoning)`, parser.rs:3319) — so `add_opencode_tokens` adds the object's own `output` and its `reasoning` BOTH into `output_tokens` (both billed at the output rate), while writing its own `reasoning_tokens` figure for the breakdown (rm-619, 2026-10-11; review fix F1, same day). Never REPLACE output with reasoning: the pre-review `Some(reasoning)` arm did exactly that and billed a mixed {output:50, reasoning:30} message at 30 output tokens — below even the no-reasoning control (60).
- **Breakdown OF output → never additive.** codex `reasoning_output_tokens` is a decomposition of the already-counted output — it stays a separately-visible figure beside output and must NOT be added into it (rm-617 discipline).

## Failure mode if skipped

The pricing loop reads specific bucket keys. An unfolded-but-billed quantity renders $0.0000 for real spend (the rm-619 twin-PoC shape: 777 real reasoning tokens → 2 text-estimated output tokens, cost 0.0); an over-folded quantity double-bills. Neither crashes — both are silent accounting lies that only a red-first twin fixture (same session, ± the field) can catch.

## Procedure

1. Read the producer's schema for the field (source of truth: the producer's own persistence code, not the dossier summary — the ae668616 mint corrected exactly such a citation drift).
2. Decide fold direction by the rule above; record the oracle in the row/EXECUTED bullet.
3. Pin with twin fixtures under `tests/fixtures/usage-accounting/` — control vs twin differing only in the field under test, red-first, both token counts and cost asserted.
