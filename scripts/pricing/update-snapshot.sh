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
                          offline from it instead of fetching
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
    cp -- "$src" "$tmp"
    echo "refresh source: captured file $src (offline refresh)"
else
    curl -fsSL "$url" -o "$tmp"
    echo "refresh source: $url (live fetch)"
fi

# ---- rm-1532 extract guard (rider, run ab7a7357 cycle 5, batch
# 'pricing-truth cure v2') ------------------------------------------------
# The embedded models.dev first-party extract is FROZEN dated state:
# this script refreshes ONLY the LiteLLM bundle. Two loud refusals up
# front, one after the write:
#   (1) pricing.rs MODELS_DEV_SNAPSHOT_DATE must equal the extract's
#       _snapshot.date (the desync analog of the PRICING_SNAPSHOT_DATE
#       gate drift-check enforces on the bundle);
#   (2) the pinned live catalog must price NONE of the extract's dated
#       ids first-party — a first-party row SUPERSEDES the extract and
#       demands a deliberate re-adjudication (rm-1532 self-retirement),
#       never a silent shadowing;
#   (3) the extract file must be byte-identical after the refresh (the
#       md5 check below the writer) — a refresh path that ever
#       regenerates or rewrites it is a defect, not a feature.
extract="crates/agenttrace-core/src/models_dev_snapshot.json"
extract_md5_before="$(md5sum "$extract" | awk '{print $1}')"
python3 - "$tmp" "$extract" <<'GUARD'
import json
import re
import sys

live_path, extract_path = sys.argv[1], sys.argv[2]


def costed_chat_keys(data):
    # Same keep-filter basis drift-check.sh reports on, so the refusal
    # fires on exactly the rows the catalog would serve.
    keep = set()
    for key, value in data.items():
        if not isinstance(value, dict) or "*" in key:
            continue
        if value.get("mode") != "chat":
            continue
        try:
            priced = (
                float(value.get("input_cost_per_token") or 0) > 0
                or float(value.get("output_cost_per_token") or 0) > 0
            )
        except (TypeError, ValueError):
            continue
        if priced:
            keep.add(key)
    return keep


priced_live = costed_chat_keys(json.load(open(live_path)))
extract = json.load(open(extract_path))
extract_date = extract.get("_snapshot", {}).get("date")

pricing_rs = open("crates/agenttrace-core/src/pricing.rs").read()
match = re.search(
    r'const MODELS_DEV_SNAPSHOT_DATE: &str = "([0-9]{4}-[0-9]{2}-[0-9]{2})"',
    pricing_rs,
)
if not match:
    sys.exit(
        "REFUSED: MODELS_DEV_SNAPSHOT_DATE declaration not found in "
        "crates/agenttrace-core/src/pricing.rs — the rm-1532 extract date "
        "const moved; update this guard's regex before refreshing."
    )
if match.group(1) != extract_date:
    sys.exit(
        f"REFUSED: pricing.rs MODELS_DEV_SNAPSHOT_DATE ({match.group(1)}) and "
        f"models_dev_snapshot.json _snapshot.date ({extract_date}) disagree — "
        "the extract is frozen dated state (rm-1532); resync them deliberately, "
        "then re-run the refresh."
    )

superseded = sorted(
    qual.split("/", 1)[1]
    for qual in extract
    if qual != "_snapshot" and qual.split("/", 1)[1] in priced_live
)
if superseded:
    sys.exit(
        "REFUSED: live LiteLLM now prices first-party row(s) the models.dev "
        "extract also serves: "
        + ", ".join(superseded)
        + " — such a row SUPERSEDES the extract (rm-1532 self-retirement); "
        "re-adjudicate the extract (drop the row, bump its _snapshot.date + "
        "the MODELS_DEV_SNAPSHOT_DATE const, fold the new digest through "
        "catalog_identity) instead of letting the refresh silently shadow it."
    )
row_count = len([qual for qual in extract if qual != "_snapshot"])
print(
    f"rm-1532 extract guard: frozen extract intact ({row_count} dated rows, "
    f"date {extract_date}, const in sync, no first-party LiteLLM rows)"
)
GUARD

python3 - "$tmp" "$out" "$date" "$drops" <<'EOF'
import json
import os
import sys

src, dst, date, drops_path = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]

RATE_FIELDS = (
    "input_cost_per_token",
    "output_cost_per_token",
    "cache_creation_input_token_cost",
    "cache_read_input_token_cost",
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
adds = sorted(set(live) - set(prior))
rate_mutations = sorted(
    k for k in set(live) & set(prior) if any(live[k].get(f) != prior[k].get(f) for f in RATE_FIELDS)
)
window_mutations = sorted(
    k
    for k in (set(live) & set(prior)) - set(rate_mutations)
    if live[k].get("max_input_tokens") != prior[k].get("max_input_tokens")
)

snapshot = {
    "_snapshot": {
        "source": "BerriAI/litellm model_prices_and_context_window.json",
        "date": date,
        "models": len(merged),
        # Keys that survived ONLY through the never-drop union: upstream
        # no longer prices them, the committed bundle did. rm-588.
        "retained_from_previous": retained,
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
    f"{len(prior)} pre-refresh | +{len(adds)} added | {len(rate_mutations)} rate-mutated | "
    f"{len(window_mutations)} window-only-changed | {len(retained)} retained-by-union | "
    f"{honored} manual drop(s)"
)
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

# rm-1532 extract guard, arm (3): the refresh must leave the frozen
# models.dev extract byte-identical (see the block above the writer).
extract_md5_after="$(md5sum "$extract" | awk '{print $1}')"
if [[ "$extract_md5_before" != "$extract_md5_after" ]]; then
    echo "REFUSED: $extract changed during the refresh" >&2
    echo "  (md5 $extract_md5_before -> $extract_md5_after). The extract is" >&2
    echo "  frozen dated state (rm-1532): restore it from git and re-run;" >&2
    echo "  regenerate it ONLY through a deliberate re-adjudication, never" >&2
    echo "  as a side effect of a LiteLLM refresh." >&2
    exit 1
fi
