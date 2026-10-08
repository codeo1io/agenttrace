# opencode fork marker (parentID / parent_id) — pinned survey

Status: implemented (rm-548; sqlite-lane scope superseded by rm-791 at
integration 2026-10-08 — see "Where agenttrace implements the exclusion").
Pinned 2026-10-06 from opencode `dev` source and the ccusage fix for the
same class; re-verify against `packages/core/src/session/info.ts` when the
discovery lane changes.

## The bug class

opencode lets a user fork a session (`opencode --fork` / the UI fork action).
A forked session's message directory starts as a **replay of the parent's
history**: the fork re-emits the parent's messages as its own rows. Any usage
aggregator that walks every session and sums tokens therefore double-counts
everything the parent did, once per fork.

ccusage hit exactly this and shipped the same exclusion in v20.0.26
(issue #1782, "opencode forked sessions are double counted"; fix commit
2b1ee578 adds `fork_session_id`/`fork_boundary` handling in their sqlite
reader). agenttrace reads opencode through two lanes and implemented the
exclusion in both — then the lanes SPLIT at integration (see the update
below): live host measurement (run 2023f222, rm-791) showed the sqlite
lane's `parent_id` rows on real installs are subagent children with their
own message rows, not fork replays.

## The marker, pinned from source

| Lane | Marker | Source of truth |
|---|---|---|
| JSON storage (`…/<data>/opencode/storage/session/info/<sid>.json`) | `"parentID": "<parent session id>"` (non-empty) | `packages/core/src/session/info.ts` — `Session.Info` schema, field `parentID: string \| undefined` (line 19 at pin time) |
| sqlite (`…/<data>/opencode/opencode.db`) | `session.parent_id` column, non-null | `packages/core/src/session/sql.ts` — `SessionTable.parent_id` with index `session_parent_idx` |

In the JSON storage lane both properties denote the same edge: "this session
was forked FROM that parent". In the sqlite lane the column name is shared
but the live-populated edge is the subagent parent (rm-791) — see the split
below. A missing/absent field (older opencode, non-forked session) means
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
- sqlite lane: SUPERSEDED at integration (2026-10-08, run 2023f222,
  conflict case 0da4ace218e64e45a6401ff18aaf6074, rm-791 outranking
  rm-548's sqlite arm per the authority order): `session.parent_id` rows
  are treated as subagent CHILDREN — selected leniently (feature-detected,
  like the stored-total columns) in `opencode_sqlite_session_rows`, parked
  in `Metrics::parent_session` as the raw parent row id, and resolved by
  `attribute_subagents` (`subagents.rs`) via `Metrics::session_key` inside
  the same database into `subagent_count`/`subagent_cost`/
  `subagent_tokens` rollups that stay separate from the parent's own
  usage. Ground truth for the supersession: on the reference host 102 of
  310 opencode sessions carry `parent_id`, and sampled children own
  message rows with ids distinct from their parents' (e.g. child
  `ses_ff744ec59…` with 7 own message rows vs its parent's 56) — an
  attribution defect, not a double-count. The JSON lane's evidence
  (storage-schema replay semantics, pinned above from `info.ts` and the
  ccusage #1782 release note) was pinned to the `parentID` storage doc and
  its exclusion stands unchanged.
- The JSON lane's count funnels into `LoadReport.opencode_fork_excluded`
  and surfaces via the rm-436/437 disclosures channel
  (`data_health_scoped(...).disclosures["opencode_fork_excluded_sessions"]`)
  — the exclusion is never silent. The sqlite lane contributes 0 to that
  counter after the supersession (its children are retained and
  attributed; a fork-shaped sqlite corpus would now be a NEW finding to
  survey, not a silently-excluded row).

## Golden corpus shape

`tests/discovery_contract.rs` (`opencode_json_fork_copies_excluded…`,
`opencode_explicit_dir_still_renders_forked_copy`, and — rewritten at the
rm-791 supersession —
`opencode_db_parent_id_rows_retain_and_attribute_across_warm_snapshots`)
seeds exactly:

- parent `ses-parent` (no marker) + independent `ses-other` (no marker) +
  fork `ses-fork` (`"parentID": "ses-parent"` / `parent_id='ses-parent'`);
- JSON lane, auto-discovery loads **2** (fork excluded, count disclosed as 1);
- explicit `-d` on the storage root loads **3** (fork renders);
- sqlite lane, both rows load (**2**), the child links and rolls into the
  parent, and the lane's fork-excluded count stays **0** warm and cold.
