# workbuddy usage-basis contract fixtures (rm-497, run 1f12309adc31 cycle 3)

These two journals pin the workbuddy `input_tokens` **basis contract** — the
parser nets reported input against `cache_read_input_tokens`
(`saturating_sub().max(0)`), which is correct only when the vendor reports
input GROSS of cache reads. Upstream luoyuctl/agenttrace issue #310 names the
same basis question; at the fork point the netting clamp was silent.

- `basis-includes.jsonl` — the contract holds: input 175 reported gross, 40
  served from cache → net input 135, no diagnostics.
- `basis-clamped.jsonl` — the contract breaks: input 50 reported net (or the
  vendor cache counter is wrong), 5000 claimed cached → the netting clamps
  input to 0. Recorded live at ea5c41e: `tokens 5040 (input silently 0), cost
  priced on cache alone, zero counters`. The parser keeps the netting
  semantics but must fire `workbuddy_usage_basis_clamped` in per-session
  parse diagnostics (the rm-400/rm-401 channel) so the zero is a visible
  decision, not a silent loss.

Both files must stay ≥2 lines: single-document `.jsonl` files skip the
JsonlProbe array entirely (`parser.rs` fast path), and workbuddy detection
requires a `function_call`/`function_call_result`/`reasoning` entry carrying
`sessionId` AND `cwd`.
