# workbuddy usage-basis contract fixtures (rm-497, run 1f12309adc31 cycle 3)

These two journals pin the workbuddy `input_tokens` **basis contract** —
the parser nets reported input against `cache_read_input_tokens`, which is
correct only when the vendor reports input GROSS of cache reads. Upstream
luoyuctl/agenttrace issue #310 names the same basis question; at the fork
point the netting clamp was silent.

Merged semantics (conflict case 739e7bc4): the landed upstream #316 clamp
(`subtract_cached_input`) clamps the CACHE count to the remaining input, and
the whole `workbuddy_input_basis:*` family lands on
`Metrics.disclosure_counters` (the rm-538 non-loss channel — rendered under
"Disclosed facts", never in `line_skips`). Usage blocks riding
`function_call_result` lines stay uncounted (they echo the request whose
usage the message/function_call record already summed) but fire
`workbuddy_usage_dropped:function_call_result` on the same channel;
reasoning-record usage is READ into the sum (upstream #311).

- `basis-includes.jsonl` — the contract holds: input 175 reported gross, 40
  served from cache → net input 135, no diagnostics.
- `basis-clamped.jsonl` — the contract breaks: input 50 reported net (or the
  vendor cache counter is wrong), 5000 claimed cached → the clamp reads
  input 0 / cache_r 50. Recorded live at ea5c41e: `tokens 5040 (input
  silently 0), cost priced on cache alone, zero counters`. The parser keeps
  the netting semantics but fires `workbuddy_input_basis:cache_clamped` on
  the disclosure_counters channel so the clamp is a visible decision, not a
  silent loss.

Both files must stay ≥2 lines: single-document `.jsonl` files skip the
JsonlProbe array entirely (`parser.rs` fast path), and workbuddy detection
requires a `function_call`/`function_call_result`/`reasoning` entry carrying
`sessionId` AND `cwd`.
