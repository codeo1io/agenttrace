#!/usr/bin/env bash
# Regenerates crates/agenttrace-core/src/pricing_snapshot.json — the vendored
# offline pricing catalog — through the rm-588 never-drop union contract:
# every model priced by the previously committed bundle stays priced after
# the refresh unless scripts/pricing/snapshot-drops.txt carries a dated
# manual-drop note for it. A refresh whose result would lose a priced key
# REFUSES to write and exits non-zero (shrink-check), so an upstream removal
# or rename can never silently vanish a price the bundle once carried.
# Wildcard glob keys (rm-903) — `bedrock/*/1-month-commitment/…` — are
# skipped on the live side: the lookup is exact-key, so a glob row can never
# match; concrete-region commitment keys stay (exact-matchable, priced).
#
# rm-176 pricing-truth wave (2026-10-21): the refresh stamps PROVENANCE into
# _snapshot — `upstream_commit` (the LiteLLM commit the vendored bytes were
# fetched from) and `fetched_at` (when this refresh fetched them) — and a
# DATED RETIREMENT LEDGER (`_snapshot.retired_ledger`: retained key -> first
# refresh date that observed it retired from live). drift-check.sh's
# removal guard exempts exactly the ledgered keys, so the guard means
# "retired since the last sanctioned refresh", not "retired ever" — the
# distinction that wedged pricing-drift runs 38259897728 and 39246601177
# into permanent red.
#
# Run from the repository root, then commit the result, update
# PRICING_SNAPSHOT_DATE in crates/agenttrace-core/src/pricing.rs to match
# the printed date, and append the printed added/removed delta to the
# rm-176 row in ROADMAP.md (rm-588 acceptance clause 3).
#
# The refresh takes NO arguments: no-args runs it; --help/-h prints usage;
# any other argument is REFUSED, so a stray invocation can never rewrite
# the tracked bundle.
#
# Environment overrides:
#   LITELLM_SNAPSHOT_SRC  path to a captured LiteLLM catalog file; refreshes
#                         offline from it instead of fetching (deterministic
#                         re-runs; verify the capture is recent enough first)
#   LITELLM_UPSTREAM_COMMIT  LiteLLM commit the catalog came from (online
#                         mode resolves+fetches BY that commit; offline mode
#                         REQUIRES it — there is no fetch to hang it on)
#   LITELLM_FETCHED_AT   override for the _snapshot.fetched_at stamp
#                         (deterministic re-runs; default: run time, UTC)
#   SNAPSHOT_DATE         date stamped into _snapshot.date and compared
#                         against manual-drop notes (default: today UTC)
set -euo pipefail

url="https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json"
out="crates/agenttrace-core/src/pricing_snapshot.json"
drops="scripts/pricing/snapshot-drops.txt"
date="${SNAPSHOT_DATE:-$(date -u +%F)}"
src="${LITELLM_SNAPSHOT_SRC:-}"

usage() {
    cat <<'USAGE'
Regenerates crates/agenttrace-core/src/pricing_snapshot.json (the vendored
offline pricing catalog) through the rm-588 never-drop union contract.
Live-source keys containing '*' (glob rows) are skipped (rm-903): the
offline lookup is exact-key and can never match them.

Usage:
    scripts/pricing/update-snapshot.sh          run the refresh (no arguments)
    scripts/pricing/update-snapshot.sh --help   print this help

Any other argument is REFUSED: the refresh must never be triggered by a
stray invocation (accidental --help, a tab-completed flag, a typo), because
it rewrites a tracked file.

Environment overrides:
    LITELLM_SNAPSHOT_SRC  path to a captured LiteLLM catalog file; refreshes
                          offline from it instead of fetching (requires
                          LITELLM_UPSTREAM_COMMIT for the provenance stamp)
    LITELLM_UPSTREAM_COMMIT  LiteLLM commit the catalog came from (online
                          mode resolves the latest commit touching the
                          catalog path and fetches the bytes BY that sha)
    LITELLM_FETCHED_AT   override for the _snapshot.fetched_at stamp
                          (deterministic re-runs; default: run time, UTC)
    SNAPSHOT_DATE         date stamped into _snapshot.date and compared
                          against manual-drop notes (default: today UTC)

After a refresh: update PRICING_SNAPSHOT_DATE in pricing.rs, append the
printed added/removed delta to the rm-176 row in ROADMAP.md (rm-588 clause 3),
and commit the result.
USAGE
}

case "${1:-}" in
    "")
        ;; # no arguments: the documented refresh path
    --help | -h)
        usage
        exit 0
        ;;
    *)
        usage >&2
        echo "REFUSED: unknown argument '$1' — the refresh takes no arguments" >&2
        exit 2
        ;;
esac

tmp="$(mktemp)"
trap 'rm -f "$tmp" "$out.new"' EXIT

if [[ -n "$src" ]]; then
    # Offline refresh from a captured catalog: the provenance commit MUST
    # arrive by env — there is no fetch to resolve it from. This is the
    # pricing-drift workflow's path (its single gate fetch captures catalog
    # + commit together, preserving review-fix-F4's no-TOCTOU contract).
    commit="${LITELLM_UPSTREAM_COMMIT:-}"
    if [[ -z "$commit" ]]; then
        echo "REFUSED: offline refresh (LITELLM_SNAPSHOT_SRC set) needs LITELLM_UPSTREAM_COMMIT — the LiteLLM commit that produced the captured catalog (the pricing-drift workflow exports it at its single gate fetch)" >&2
        exit 2
    fi
    catalog_url="$url"
    cp -- "$src" "$tmp"
    echo "refresh source: captured file $src (offline refresh, upstream $commit)"
else
    # Online refresh resolves the latest commit touching the catalog path
    # FIRST, then fetches the catalog BY that commit sha — the vendored bytes
    # are by construction the bytes of the stamped `upstream_commit` (no
    # raw-`main` fetch whose bytes could outrun the stamp).
    commit="$(curl -fsSL --max-time 30 "https://api.github.com/repos/BerriAI/litellm/commits?path=model_prices_and_context_window.json&per_page=1" | python3 -c 'import json,sys; print(json.load(sys.stdin)[0]["sha"])')" \
        || { echo "REFUSED: could not resolve the upstream commit for the provenance stamp (GitHub API fetch failed)" >&2; exit 2; }
    catalog_url="https://raw.githubusercontent.com/BerriAI/litellm/${commit}/model_prices_and_context_window.json"
    curl -fsSL --max-time 60 "$catalog_url" -o "$tmp"
    echo "refresh source: $catalog_url (live fetch at commit $commit)"
fi
fetched_at="${LITELLM_FETCHED_AT:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"

python3 - "$tmp" "$out" "$date" "$drops" "$commit" "$fetched_at" <<'EOF'
import json
import os
import sys

src, dst, date, drops_path, upstream_commit, fetched_at = sys.argv[1:7]

RATE_FIELDS = (
    "input_cost_per_token",
    "output_cost_per_token",
    "cache_creation_input_token_cost",
    "cache_read_input_token_cost",
    # rm-176 pricing-truth wave (2026-10-21): LiteLLM now publishes two
    # further per-token COST axes. They ride through trim() under the
    # rm-1508 omit-when-absent contract and count as rate mutations here
    # so upstream repricing on them is gate-visible drift, not silent.
    "explicit_cache_read_input_token_cost",
    "output_cost_per_reasoning_token",
)


def trim(data):
    # Same trim as always: chat-mode entries with at least one nonzero
    # per-token rate, keeping only the fields the offline catalog carries.
    # Vendor context window and deprecation date ride along when the
    # source carries them (rm-231 / rm-419).
    keep = {}
    for key, value in data.items():
        if not isinstance(value, dict):
            continue
        if value.get("mode") != "chat":
            continue
        # rm-903: LiteLLM publishes glob keys (bedrock/*/1-month-commitment/
        # …, 4 of them in the 2026-10-08 refresh). Our lookup is exact-key,
        # so such a row can never match — skip it at refresh time instead of
        # vendoring unmatchable dead weight. Concrete-region commitment-tier
        # keys (bedrock/us-east-1/1-month-commitment/…) deliberately stay:
        # they are exact-matchable strings and priced rows in their own
        # right. Under the rm-588 union the filter sits on the live side
        # only: the committed bundle carries zero wildcard keys (pinned by
        # tests/pricing_snapshot_hygiene.rs
        # vendored_pricing_bundle_carries_no_wildcard_keys), so the union
        # can never re-admit one. A hypothetical wildcard key already in
        # a prior bundle would be union-RETAINED (the shrink-check only
        # fires on lost keys), so the fail-closed signal is that same
        # hygiene pin: the refresh stays writable but the bundle fails
        # the zero-wildcard test until the key leaves through a dated
        # drop note in snapshot-drops.txt (fail-closed, never fail-open).
        # The drift gate mirrors this filter (drift-check.sh kept(),
        # rm-903): live glob rows count on NEITHER side, or the gate
        # would report permanent phantom new_costed drift that this
        # refresh can never clear.
        if "*" in key:
            continue
        inp = value.get("input_cost_per_token") or 0
        outp = value.get("output_cost_per_token") or 0
        if inp == 0 and outp == 0:
            continue
        entry = {
            "input_cost_per_token": inp,
            "output_cost_per_token": outp,
            "mode": "chat",
            "litellm_provider": value.get("litellm_provider") or "",
        }
        # rm-1508 (run 5c144613 cycle 4): preserve cache-rate ABSENCE.
        # The `or 0` zero-fill destroyed absent-vs-published-$0 before
        # the Rust ingest could ever see it (51.5%/84.3% of the
        # 2026-10-08 bundle carried a silent $0 cache-read/write rate
        # for rows that publish none). The key rides along only when
        # the source carries a number — an explicit 0 stays a published
        # $0 — mirroring the flex/ultrafast omit-when-absent contract
        # below. NOTE for the next refresh (the rm-176 reconcile lane):
        # keys that survive only through the never-drop union carry
        # their pre-fix zero-filled values forward and age out as
        # upstream drops them; live-side rows get the distinction, and
        # tests/pricing_snapshot_hygiene.rs pins the counts so any
        # movement is conscious.
        for cache_field in (
            "cache_creation_input_token_cost",
            "cache_read_input_token_cost",
        ):
            rate = value.get(cache_field)
            if isinstance(rate, (int, float)) and not isinstance(rate, bool):
                entry[cache_field] = rate
        # rm-176 pricing-truth wave (2026-10-21): carry the two NEW LiteLLM
        # cost axes under the same omit-when-absent contract —
        # `explicit_cache_read_input_token_cost` (qwen3.7-max family: cache
        # reads bill at an explicit rate DISTINCT from
        # cache_read_input_token_cost) and `output_cost_per_reasoning_token`
        # (qwen dashscope family: reasoning output bills at its own rate).
        # Absent stays absent; a published 0 stays a published 0. At this
        # refresh 3 live rows carry the explicit-cache-read axis and 68 the
        # reasoning-output axis (census in the phase record; the Rust ingest
        # + cache_rate_census enumerate them so the counts stay pinned).
        for axis_field in (
            "explicit_cache_read_input_token_cost",
            "output_cost_per_reasoning_token",
        ):
            axis = value.get(axis_field)
            if isinstance(axis, (int, float)) and not isinstance(axis, bool):
                entry[axis_field] = axis
        max_input = value.get("max_input_tokens")
        if isinstance(max_input, int) and max_input > 0:
            entry["max_input_tokens"] = max_input
        deprecation = value.get("deprecation_date")
        if isinstance(deprecation, str) and deprecation.strip():
            entry["deprecation_date"] = deprecation.strip()
        # Flex-tier rates ride along when the source carries them (rm-610,
        # run 36c9140f). Zero/absent values are omitted so "carries flex
        # pricing" stays greppable — a flex turn of a row without them
        # bills at the standard rates above.
        for flex_field in (
            "input_cost_per_token_flex",
            "output_cost_per_token_flex",
            "cache_creation_input_token_cost_flex",
            "cache_read_input_token_cost_flex",
        ):
            flex = value.get(flex_field)
            if isinstance(flex, (int, float)) and flex > 0:
                entry[flex_field] = flex
        # Ultrafast-tier rates ride along under the identical contract
        # (rm-164 cycle-2 rider, run b5d391ec — restored at integration
        # conflict case bc360d0d): without this arm the next refresh
        # would silently strip the `*_ultrafast` fields the bundled
        # snapshot and the pricing ingest both carry, regressing
        # ultrafast turns back to standard-rate billing. Zero/absent
        # values are omitted (same greppability rule as flex);
        # `_above_272k_tokens_ultrafast` long-context variants are not
        # carried (the rm-610 decision, held for the third tier family).
        for ultrafast_field in (
            "input_cost_per_token_ultrafast",
            "output_cost_per_token_ultrafast",
            "cache_creation_input_token_cost_ultrafast",
            "cache_read_input_token_cost_ultrafast",
        ):
            ultrafast = value.get(ultrafast_field)
            if isinstance(ultrafast, (int, float)) and ultrafast > 0:
                entry[ultrafast_field] = ultrafast
        keep[key] = entry
    return keep


def manual_drops(path, today):
    # rm-588 manual-drop gate: a previously priced key may leave the bundle
    # ONLY through a note "<YYYY-MM-DD> <exact-bundle-key> <reason...>".
    # Notes dated after the snapshot date do not fire yet; malformed lines
    # refuse the refresh outright (fail-closed, never fail-open).
    drops = {}
    try:
        with open(path) as f:
            lines = f.read().splitlines()
    except FileNotFoundError:
        return drops
    for line in lines:
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split(None, 2)
        if len(parts) < 3 or len(parts[0]) != 10:
            sys.exit(f"REFUSED: malformed drop note (want 'YYYY-MM-DD <key> <reason>'): {line!r}")
        when, key, reason = parts
        try:
            int(when.replace("-", ""))
        except ValueError:
            sys.exit(f"REFUSED: malformed drop date {when!r} in note: {line!r}")
        if when > today:
            print(f"drop note not yet effective (dated {when}, snapshot {date}): {key}")
            continue
        drops[key] = (when, reason)
    return drops


live = trim(json.load(open(src)))
prior_doc = json.load(open(dst))
prior = {k: v for k, v in prior_doc.items() if k != "_snapshot"}

# Vacuous-refresh floor (review hardening on rm-588): a degenerate live
# source — valid JSON that prices nothing, or an implausibly small share of
# the prior catalog — must REFUSE instead of stamping a fresh _snapshot.date
# on 100% stale prior content carried by the union merge.
overlap = [k for k in prior if k in live]
if not live:
    sys.exit(
        "REFUSED: live source priced 0 models (vacuous refresh; would stamp a "
        "fresh date on the stale prior)"
    )
if len(overlap) * 2 < len(prior):
    sys.exit(
        f"REFUSED: live source prices only {len(overlap)}/{len(prior)} prior "
        "models (<50% overlap; refusing an implausibly partial refresh)"
    )

# Union merge (rm-588 arm 1): start from the trimmed live catalog, then
# carry every previously priced key live no longer provides as-is.
merged = dict(live)
for key in sorted(prior):
    if key not in merged:
        merged[key] = prior[key]

drops = manual_drops(drops_path, date)
honored = 0
for key, (when, reason) in sorted(drops.items()):
    if key not in merged:
        print(f"drop note references a key not in the merge (inert): {key}")
        continue
    if key in live:
        print(f"drop note for a key still priced live (ignored this refresh): {key}")
        continue
    del merged[key]
    honored += 1
    print(f"manual drop honored ({when}): {key} — {reason}")

# Shrink-check (rm-588 arm 2): the post-refresh priced set may never lose
# entries vs the committed bundle except through honored dated drop notes.
# The union merge above makes this hold by construction; the check exists
# so that any future edit reintroducing a wholesale regeneration (the exact
# bug this contract closed) refuses to write instead of silently dropping.
lost = sorted(set(prior) - set(merged))
uncovered = [k for k in lost if k not in drops]
if uncovered:
    shown = ", ".join(uncovered[:10])
    sys.exit(
        f"REFUSED: refresh would drop {len(uncovered)} priced model(s) without a "
        f"dated manual-drop note in {drops_path}: {shown}"
    )
floor = len(prior) - honored
if len(merged) < floor:
    sys.exit(
        f"REFUSED: post-refresh priced-model count {len(merged)} shrank below the "
        f"union floor {floor} (pre-refresh {len(prior)}, honored drops {honored})"
    )

retained = sorted(k for k in prior if k not in live and k in merged)

# Dated retirement ledger (rm-176 pricing-truth wave): every key the union
# retains against a live catalog that no longer prices it, stamped with the
# FIRST refresh date that observed the retirement (sticky — carried from
# the prior bundle's ledger when the key was already retired there) and
# pruned to keys still in the merge (a key that returned live, or left via
# a dated drop note, stops being ledgered). drift-check.sh's removal guard
# exempts exactly these keys: the guard means "retired since the last
# sanctioned refresh", not "retired ever".
prior_meta = prior_doc.get("_snapshot") or {}
prior_ledger = prior_meta.get("retired_ledger") or {}
if not isinstance(prior_ledger, dict) or not all(
    isinstance(k, str) and isinstance(v, str) for k, v in prior_ledger.items()
):
    sys.exit(
        "REFUSED: prior bundle _snapshot.retired_ledger is malformed "
        "(want {key: YYYY-MM-DD} strings)"
    )
ledger = {
    k: v
    for k, v in prior_ledger.items()
    if k in merged and k not in live
}
for key in retained:
    prior_date = prior_ledger.get(key)
    ledger[key] = min(prior_date, date) if prior_date else date
adds = sorted(set(live) - set(prior))
# Rate mutations on the GATE basis (drift-check.sh parity: absence reads as
# 0), so this count is REPRICINGS — the first rm-1508-preserving refresh
# otherwise reports the zero-filled -> absent transition of the pre-fix
# bundle as thousands of phantom "mutations". That transition is real
# movement though, so it is counted and reported separately below.
rate_mutations = sorted(
    k
    for k in set(live) & set(prior)
    if any(float(live[k].get(f) or 0) != float(prior[k].get(f) or 0) for f in RATE_FIELDS)
)
zero_to_absent = sorted(
    k
    for k in (set(live) & set(prior)) - set(rate_mutations)
    if any((live[k].get(f) is None) != (prior[k].get(f) is None) for f in RATE_FIELDS)
)
window_mutations = sorted(
    k
    for k in (set(live) & set(prior)) - set(rate_mutations)
    if live[k].get("max_input_tokens") != prior[k].get("max_input_tokens")
)

snapshot = {
    "_snapshot": {
        "source": "BerriAI/litellm model_prices_and_context_window.json",
        # rm-176 provenance stamps: the exact upstream commit the vendored
        # bytes were fetched from, and when this refresh fetched them —
        # "which LiteLLM state are we paying from" is answerable from the
        # vendored file alone.
        "upstream_commit": upstream_commit,
        "date": date,
        "fetched_at": fetched_at,
        "models": len(merged),
        # Keys that survived ONLY through the never-drop union: upstream
        # no longer prices them, the committed bundle did. rm-588.
        "retained_from_previous": retained,
        # Retained keys -> first refresh date that observed the retirement
        # (the removal-guard exemption basis in drift-check.sh). rm-176.
        "retired_ledger": ledger,
    }
}
snapshot.update(merged)
# Atomic write (rm-693 discipline, review hardening): dump to a sibling
# temp and os.replace into place, so a crash mid-dump can never leave the
# tracked bundle truncated.
tmp_dst = dst + ".new"
try:
    with open(tmp_dst, "w") as f:
        json.dump(snapshot, f, separators=(",", ":"), sort_keys=True)
        f.write("\n")
    os.replace(tmp_dst, dst)
except BaseException:
    try:
        os.remove(tmp_dst)
    except FileNotFoundError:
        pass
    raise

print(
    f"wrote {len(merged)} chat models to {dst} (snapshot date {date}): "
    f"{len(prior)} pre-refresh | +{len(adds)} added | {len(rate_mutations)} repriced | "
    f"{len(zero_to_absent)} zero-to-absent (rm-1508) | {len(window_mutations)} window-only-changed | "
    f"{len(retained)} retained-by-union | {len(ledger)} ledgered-retired | {honored} manual drop(s)"
)
if upstream_commit:
    print(f"provenance: upstream commit {upstream_commit}, fetched {fetched_at}")
if adds:
    sample = ", ".join(adds[:8])
    print(f"added (first 8): {sample}")
if retained:
    print(f"retained by union: {', '.join(retained)}")
print(
    f"next: set PRICING_SNAPSHOT_DATE in crates/agenttrace-core/src/pricing.rs "
    f"to {date}, then append the added/removed delta above to the rm-176 row "
    "in ROADMAP.md (rm-588 clause 3)"
)
EOF
