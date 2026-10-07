# opencode fork marker (parentID / parent_id) — pinned survey

Status: implemented (rm-548). Pinned 2026-10-06 from opencode `dev` source and
the ccusage fix for the same class; re-verify against
`packages/core/src/session/info.ts` when the discovery lane changes.

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

## Golden corpus shape

`tests/discovery_contract.rs` (`opencode_json_fork_copies_excluded…`,
`opencode_explicit_dir_still_renders_forked_copy`,
`opencode_db_fork_copies_excluded_and_disclosed`) seeds exactly:

- parent `ses-parent` (no marker) + independent `ses-other` (no marker) +
  fork `ses-fork` (`"parentID": "ses-parent"` / `parent_id='ses-parent'`);
- auto-discovery loads **2** (fork excluded, count disclosed as 1);
- explicit `-d` on the storage root loads **3** (fork renders);
- warm snapshot cache still reports the exclusion count (sqlite lane).
