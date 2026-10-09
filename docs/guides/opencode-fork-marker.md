# opencode fork marker (parentID / parent_id) — pinned survey

Status: implemented (rm-548; boundary-aware accounting rm-805). Pinned
2026-10-06 from opencode `dev` source and the ccusage fix for the same class;
re-verify against `packages/core/src/session/info.ts` when the discovery lane
changes.

## The bug class

opencode lets a user fork a session (`opencode --fork` / the UI fork action).
A forked session's message directory starts as a **replay of the parent's
history**: the fork re-emits the parent's messages as its own rows. Any usage
aggregator that walks every session and sums tokens therefore double-counts
everything the parent did, once per fork.

ccusage hit exactly this and shipped the same exclusion in v20.0.26
(issue #1782, "opencode forked sessions are double counted"; fix commit
2b1ee578 adds `fork_session_id`/`fork_boundary` handling in their sqlite
reader). agenttrace reads opencode through two lanes and needs the exclusion
in both.

## The marker, pinned from source

| Lane | Marker | Source of truth |
|---|---|---|
| JSON storage (`…/<data>/opencode/storage/session/info/<sid>.json`) | `"parentID": "<parent session id>"` (non-empty) | `packages/core/src/session/info.ts` — `Session.Info` schema, field `parentID: string \| undefined` (line 19 at pin time) |
| sqlite (`…/<data>/opencode/opencode.db`) | `session.parent_id` column, non-null | `packages/core/src/session/sql.ts` — `SessionTable.parent_id` with index `session_parent_idx` |

Both spellings denote the same edge: "this session was forked FROM that
parent". A missing/absent field (older opencode, non-forked session) means
"not a fork" — never an error.

Notes from the survey:

- The sqlite schema is the forward direction (v2 storage); the JSON storage
  tree is what older/current installs carry per-session docs in. ccusage's
  `session_v2.fork_session_id` is the same edge under their v2 name.
- The JSON storage doc requires `id` AND `projectID` to be recognized as a
  session doc at all (`is_opencode_storage_session_doc` in
  `crates/agenttrace-core/src/parser.rs`) — corpus fixtures must carry both.
- Message content lives in `storage/message/<sid>/*.json` with text parts in
  `storage/part/<mid>/<pid>.json` keyed by **message** id, not session id; a
  session whose messages have no text parts parses to zero events and is
  skipped by design.

## Where agenttrace implements the exclusion

- JSON lane: `opencode_session_fork_parent` in
  `crates/agenttrace-core/src/discovery.rs` — reads `parentID` from the
  session doc during enumeration (auto-discovery only; explicit `-d` /
  explicit-file loads keep the fork so it still renders).
- sqlite lane: `parent_id` is selected (feature-detected, like the
  stored-total columns) in `opencode_sqlite_session_rows` in
  `crates/agenttrace-core/src/sqlite_sessions.rs`; fork rows are dropped at
  the row boundary.
- Both counts funnel into `LoadReport.opencode_fork_excluded` and surface via
  the rm-436/437 disclosures channel
  (`data_health_scoped(...).disclosures["opencode_fork_excluded_sessions"]`)
  — the exclusion is never silent.

## Boundary-aware accounting (rm-805)

The whole-drop was deliberately coarse: a fork that CONTINUES working — new
messages of its own after the fork point — was dropped entirely (losing its
new work) or would have been kept entirely (re-counting the replayed
prefix). The boundary is the fork's own `timeCreated` (the fork point — read from
the fork's info doc by `discovery.rs::opencode_fork_boundary`; opencode
cannot fork before its parent exists), and the two lanes now split
on it:

- JSON lane: a fork whose message directory holds any record strictly AFTER
  the boundary is a **continuation** — it is KEPT, but the parser drops its
  replayed prefix (every message at or before the boundary), so the fork
  contributes only its own work; the dropped prefix's token magnitude rides
  the disclosure channel (`opencode_fork_prefix_excluded_tokens`). A fork
  with no post-boundary work is still a pure replay and stays dropped
  whole. The boundary needs the parent's info doc readable — when the
  fork `timeCreated` is unusable the scan falls back to the rm-548
  whole-drop for that fork rather than guessing. There is no separate
  "boundary known" disclosure key — a resolvable boundary shows up as the
  fork's classification itself, and the disclosure surfaces are the real,
  pinned keys: orphans kept (`opencode_fork_orphans_counted`), row drops
  (`opencode_fork_excluded_sessions` / `_tokens` / `_cost…microusd`), and
  the continuation-prefix pair (`opencode_fork_prefix_excluded_tokens` /
  `_cost…microusd`), all pinned by
  `crates/agenttrace-core/tests/opencode_fork_boundary.rs`. Warm caches
  regenerate once (sqlite snapshot schema 9, JSON dir-cache schema 34) so a
  pre-boundary-aware snapshot can never keep serving stale whole-drop
  totals.
- sqlite lane: a fork row whose `parent_id` points at a session NOT in the
  database is an **orphan** — typically the parent was deleted while the
  fork lives on. rm-548 dropped it; it is now KEPT and counted, because
  with the parent gone there is no double-count to avoid. Live-parent
  forks keep the row-boundary drop, and the drop now carries its magnitude
  (`opencode_fork_excluded_tokens`, mirroring `apply_opencode_stored_totals`:
  stored totals when the row has them, else the derived counts aggregation
  would have used).
- Warm sqlite snapshots persist the boundary-aware fields
  (`fork_excluded_tokens` / `fork_orphans` / `fork_excluded_cost_microusd`)
  with `serde(default)` — the same forward-compat pattern
  `pricing_catalog_id` used — and the review fix then moved the versions
  anyway (sqlite snapshot 8 → 9, JSON dir-cache 33 → 34): a
  totals-affecting change must invalidate warm caches or the change stays
  invisible behind stale orphan-excluded totals (the rm-230 convention),
  so pre-rm-805 snapshots regenerate once instead of loading zeroed
  fields. The governance guide's schema sentences track the live
  constants.
- Pinned by `crates/agenttrace-core/tests/opencode_fork_boundary.rs`:
  continuation keeps the fork with prefix-honest totals, orphan rows count
  with their magnitude disclosed (both the stored-totals class and the
  no-stored-columns message-derived class), and warm snapshots carry the
  boundary-aware disclosure through a full cache round-trip. Pure replay
  still drops whole — pinned where that rule landed, by the rm-548 golden
  corpus fixture
  `discovery_contract.rs::opencode_json_fork_copies_excluded_from_aggregation_and_disclosed`.

## Golden corpus shape

`tests/discovery_contract.rs` (`opencode_json_fork_copies_excluded…`,
`opencode_explicit_dir_still_renders_forked_copy`,
`opencode_db_fork_copies_excluded_and_disclosed`) seeds exactly:

- parent `ses-parent` (no marker) + independent `ses-other` (no marker) +
  fork `ses-fork` (`"parentID": "ses-parent"` / `parent_id='ses-parent'`);
- auto-discovery loads **2** (fork excluded, count disclosed as 1);
- explicit `-d` on the storage root loads **3** (fork renders);
- warm snapshot cache still reports the exclusion count (sqlite lane).
