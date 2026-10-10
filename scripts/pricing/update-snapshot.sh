#!/usr/bin/env bash
# Regenerates crates/agenttrace-core/src/pricing_snapshot.json — the vendored
# offline pricing catalog — through the rm-588 never-drop union contract:
# every model priced by the previously committed bundle stays priced after
# the refresh unless scripts/pricing/snapshot-drops.txt carries a dated
# manual-drop note for it. A refresh whose result would lose a priced key
# REFUSES to write and exits non-zero (shrink-check), so an upstream removal
# or rename can never silently vanish a price the bundle once carried.
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


MODES_KEPT = ("chat", "image_generation")


def trim(data):
    # Same trim as always: text-pricable entries with at least one
    # nonzero per-token rate, keeping only the fields the offline catalog
    # carries. Vendor context window and deprecation date ride along when
    # the source carries them (rm-231 / rm-419).
    #
    # rm-176 (2026-10-10): image_generation models that ALSO carry
    # per-token rates (e.g. gpt-image-1.5, whose text-token lanes are
    # priced like any chat model) are admitted with their truthful mode;
    # resolution/steps-qualified image keys (1024-x-1024/50-steps/...) and
    # per-image-only entries carry no per-token rates and stay out of the
    # model-id space by this very filter.
    keep = {}
    for key, value in data.items():
        if not isinstance(value, dict):
            continue
        mode = value.get("mode")
        if mode not in MODES_KEPT:
            continue
        inp = value.get("input_cost_per_token") or 0
        outp = value.get("output_cost_per_token") or 0
        if inp == 0 and outp == 0:
            continue
        entry = {
            "input_cost_per_token": inp,
            "output_cost_per_token": outp,
            "cache_creation_input_token_cost": value.get("cache_creation_input_token_cost") or 0,
            "cache_read_input_token_cost": value.get("cache_read_input_token_cost") or 0,
            "mode": mode,
            "litellm_provider": value.get("litellm_provider") or "",
        }
        max_input = value.get("max_input_tokens")
        if isinstance(max_input, int) and max_input > 0:
            entry["max_input_tokens"] = max_input
        deprecation = value.get("deprecation_date")
        if isinstance(deprecation, str) and deprecation.strip():
            entry["deprecation_date"] = deprecation.strip()
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
# rm-176 (2026-10-10): `_plan_scope` is a models.dev-sourced section, not
# derivable from the LiteLLM source — it must never be union-merged as a
# fake model row (which would corrupt the header count and the
# retained_from_previous audit). Carry it across the refresh verbatim;
# scripts/pricing/ingest-modelsdev.sh re-derives it on demand.
meta_keys = {"_snapshot", "_plan_scope"}
prior = {k: v for k, v in prior_doc.items() if k not in meta_keys}

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
if "_plan_scope" in prior_doc:
    snapshot["_plan_scope"] = prior_doc["_plan_scope"]
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
    f"wrote {len(merged)} priced models ({sum(1 for v in merged.values() if v.get('mode') == 'image_generation')} image-generation) "
    f"to {dst} (snapshot date {date}): "
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
