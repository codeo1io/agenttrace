# Prevention rule: acceptance clauses that exempt sibling lanes go stale — re-PoC the exemptions

- **Class:** roadmap craft / acceptance honesty
- **Observed:** 2026-10-05/06, run `1f12309adc31` (repository-maintenance dde7875c
  cycle 3), at base `ea5c41e` / salvage `511fb01`
- **Cost:** rm-239's acceptance clause asserted "md/html lanes remain
  byte-identical to today (they are already safe)". The clause was false at
  the base where the row was finally implemented: the assess format×surface
  matrix counted **1 / 1 / 2 / 0 raw-ESC lines** across
  `--overview -f text|markdown|html|json` on a hostile model key
  (`mod<ESC>]52;c;!<BEL>el`). Believing the clause would have shipped a fix
  that closed only the text lane — the terminal-injection defect would have
  survived in both "already safe" lanes, with a roadmap row marked done.

## What happened

The clause was written during an earlier cycle against the lanes as they then
were (or as they were believed to be). Later landings re-routed render paths —
markdown table cells gained pipe/newline rewrites (`markdown_cell`,
`markdown_inline_code`), the html lane kept its `html_escape` on scalar fields
but group keys reached raw cells — and nothing re-tested the exemption. The
exempted-safety claim rode along in the acceptance text across at least two
cycles of prioritization and integration notes.

The cycle-3 assess phase falsified it cheaply: one hostile fixture, one loop
over the four formats, `grep -c ESC`. The implement then covered the whole
family (`text_cell` sanitizes after the whitespace squash; `markdown_cell` /
`markdown_inline_code` sanitize AFTER the pipe/newline rewrites so `<br>`
survives; group-key rows By-Agent/By-Model/By-Provider/By-Task-Type wrapped)
and pinned every lane with its own test, including a losslessness pin for the
json lane so sanitization could never silently eat data
(`overview_json_lane_stays_lossless_for_control_bytes`).

## The rule

1. **An acceptance clause that exempts a lane ("X is already safe", "only Y
   needs fixing") is a claim about a moving tree. It must carry a dated PoC,
   and the PoC must be re-run at the base where the row is selected and
   implemented** — not inherited from when the clause was written.
2. The cheap falsifier is a **format×surface matrix**: build one hostile
   input, run every output format against it, count control bytes (`grep -c
   ESC`). Two minutes per surface; it is exactly what caught this.
3. When a fix lands on a family, **pin every exempted lane with its own test**
   — the exemption becomes executable truth instead of prose truth. Include a
   losslessness pin for the lanes that must NOT be transformed (json), so the
   sanitizer can never over-reach.
4. At roadmap-refresh time, flag exempting clauses older than one landing on
   the touched surface as **stale until re-PoC'd** — same discipline as
   evidence refreshes, applied to the acceptance text itself.

## See also

- Roadmap row `rm-239` (render-boundary sanitization; EXECUTED bullet records
  the falsified clause and the four lane tests).
- `docs/stewardship/2026-10-06-cycle3-compound-record-run1f12309a.md`
  (cycle-3 compound record, lesson 1).
