# range-overnight fixtures (rm-890, run ac3ac300 cycle 2)

Minimal committed forms of the assess-phase PoC corpora (run c92f079f F1/F5,
`poc-sqlite-range/home/`): sqlite lanes whose `--range`/`--since` admission
must follow LAST ACTIVITY, not session start. The shared admission predicate
(rm-694 overlap basis) admits a session whose last activity is at or after
the cutoff — so a session that STARTED before the window but ran into it
counts (README.md:192-196 contract).

- `state.db` (hermes lane, `$HOME/.hermes/state.db`) — three session rows:
  `ses-overnight` started 2026-09-19T00:00:00Z, ended 2026-10-08T12:00:00Z
  (created before the 7d cutoff of 2026-10-02T00:00:00Z, activity inside
  it), `ses-fresh` started/ended 2026-10-08T18:00/18:30Z (entirely inside),
  `ses-stale` started 2026-08-30T00:00:00Z, ended 2026-09-08T00:00:00Z
  (entirely outside — admission must still exclude it). Plus `messages`
  rows per session. sha256
  `6d9919690aa6e82d9b296d8e349b25249e225e60d492cea08cf83cba63fbcfd4`.
- `opencode.db` (opencode lane, `$HOME/.local/share/opencode/opencode.db`)
  — byte-identical copy of the assess PoC database: table `session` with
  `ses-overnight` (time_created 2026-09-19-ish, time_updated
  2026-10-08T08:23Z-ish — overnight into the window) and `ses-fresh`
  (2026-10-09T04:30Z-ish, inside). sha256
  `2d23662cc6d5a2d3d5adf63fb867f6e7eb33b6fb9b4ba394fa710508a7aaf5f3`.

The tests pin the cutoff at 2026-10-02T00:00:00Z so the fixtures stay
deterministic forever. sha256 pins per the rm-542 fixture convention.
