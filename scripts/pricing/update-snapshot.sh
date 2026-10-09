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

# rm-934: an override date must be a real calendar date in YYYY-MM-DD
# form, otherwise a typo embeds an unparseable date the drift lane
# then reports forever. The regex fixes the shape; the python3 check
# (a hard dependency of the transform below) fixes the calendar —
# 2026-13-99 and 2023-02-29 pass the shape but are not real dates,
# and 2024-02-29 is.
if [[ ! "$date" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] \
    || ! python3 -c 'import datetime, sys; datetime.date.fromisoformat(sys.argv[1])' "$date" >/dev/null 2>&1; then
    echo "error: SNAPSHOT_DATE must be a real YYYY-MM-DD calendar date (got: $date)" >&2
    exit 2
fi

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
    curl -fsSL --max-time 30 --connect-timeout 10 --max-filesize $((32 * 1024 * 1024)) "$url" -o "$tmp"
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
            "cache_creation_input_token_cost": value.get("cache_creation_input_token_cost") or 0,
            "cache_read_input_token_cost": value.get("cache_read_input_token_cost") or 0,
            "mode": "chat",
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
