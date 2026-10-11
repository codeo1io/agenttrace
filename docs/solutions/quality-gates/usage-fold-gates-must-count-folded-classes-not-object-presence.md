# Usage-fold gates must count folded classes, not object presence

**Severity class:** silent data loss (usage/billing totals) with zero disclosure.
**Origin:** run `e944a060a640` cycle 1, rm-619 (opencode message-lane fold gate), implement attempt `2ba22694` — red-first proven, landed 2026-10-11.
**Related:** `zero-usage-blocks-tolerated-at-reasoning-boundaries.md` (the codex-side reasoning-boundary tolerance of the same family).

## The failure shape

A fold routine consumed a heterogeneous `tokens` object and answered a single
boolean — "did this object contain anything?" — from the mere PRESENCE of the
object:

```rust
// before: presence == folded
fn add_opencode_tokens(usage: &mut Usage, tokens: Option<&Value>) -> bool {
    let Some(map) = tokens.and_then(|v| v.as_object()) else { return false };
    // folds input/output/cache.read/cache.write IF present …
    true // returned for ANY object, folded or not
}
```

The caller used that boolean as a suppression gate on a rescue lane:

```rust
let message_had_usage = add_opencode_tokens(&mut usage, msg.tokens);
// …part lane:
"step-finish" if !message_had_usage => { /* rescue the step's tokens */ }
```

A message carrying `tokens: {"reasoning": 777}` — or only future/unknown key
names — armed the suppression on the message lane AND disarmed the rescue lane.
Net effect: the session reported **zero usage, silently** (twin PoC: control
100/50/$0.0011, twin 0/3-estimated/$0.0000, no skip key anywhere in `-f json`).

The bug is not the fold's key set — unknown keys must stay tolerated — it is
the GATE conflating "an object was present" with "a recognized class actually
folded". Presence is cheap to observe and wrong to gate on; recognition is the
only truthful signal.

## The rule

**Any gate that decides whether a fallback/rescue lane runs must be computed
from the classes that actually folded, never from container presence.**
Return the recognition detail instead of a single bool:

```rust
// after: recognized-class semantics
struct OpencodeTokensFold { billing: bool, reasoning: bool }
fn add_opencode_tokens(usage: &mut Usage, tokens: Option<&Value>) -> OpencodeTokensFold
// part lane keys on !message_fold.billing
```

Disclosure classes (e.g. `reasoning`) fold onto their own counters
(`tokens_reasoning`, never added into billed output — the CU-20 discipline)
and MUST NOT arm a billing gate. Mirror contracts (the part lane folds its
own reasoning only when the message level did not) keep each key class
counted exactly once regardless of which lane carries it.

## The pin that holds it

A twin fixture must exist where the ONLY message-level tokens carry
non-billing keys, alongside its own step-finish part:

- `crates/agenttrace-core/tests/fixtures/usage-accounting/opencode-reasoning-rescue/`
  — reasoning-only message + unknown-key message, each with its own
  `step-finish` part; pinned by
  `journal_truth_batch::fixture_opencode_reasoning_rescue_counts_step_finish_usage`
  (post-fix totals 107 input / 58 output / reasoning 777; proven 13/1 RED
  pre-fix via git-stash swap of the parser).

When auditing any future fold gate, ask: **which key set makes this gate lie?**
If the answer is "any tolerated-but-unfolded key", the gate counts presence,
not folds — fix the predicate, and pin the twin.
