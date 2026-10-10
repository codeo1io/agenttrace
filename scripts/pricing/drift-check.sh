#!/usr/bin/env bash
# drift-check.sh — provable freshness for the bundled pricing catalog (rm-176 + assess A1 + rm-006 fold).
#
# Posture (recorded 2026-10-05, run 84c45ccd cycle 1; full table in
# docs/stewardship/2026-10-05-cycle1-implementation-record-run84c45ccd.md):
#   the fork keeps the OFFLINE-FIRST BUNDLED catalog (byte-deterministic reports, no network in the
#   hot path) and automates freshness with a SCHEDULED PR-BUMP gate. Rejected postures: ccusage-style
#   hourly auto-push (no unreviewed mutation of the money table) and upstream-style runtime fetch
#   (offline-first determinism; runtime refresh already exists as the opt-in --update-pricing cache).
#
# Basis rule (the false-alarm trap this script exists to avoid): drift is computed ONLY on the
# builder keep-filter basis — mode in (chat, image_generation) AND (input_cost_per_token > 0 OR output_cost_per_token > 0) —
# mirroring scripts/pricing/update-snapshot.sh verbatim (rm-176 2026-10-10 widened both from chat-only).
# Raw total-model comparisons against LiteLLM
# main mix in non-chat/uncosted entries and misread the bundle as drifted when it is not
# (verified 2026-10-05: raw total 4,473 vs chat-with-cost 3,099 on BOTH sides = parity).
#
# Desync gate (assess A1, run 84c45ccd): the PRICING_SNAPSHOT_DATE const in pricing.rs must equal the
# bundled snapshot's `_snapshot.date`. update-snapshot.sh's manual "then update the const" step had no
# enforcement; this check fails closed (rc 1) on any mismatch — and (review fix 2026-10-06, F1) ABSENCE
# fails closed too: a pricing.rs with no parseable `const PRICING_SNAPSHOT_DATE: &str = "<date>"`
# declaration, or a bundle with no `_snapshot.date`, is an INFRA failure (rc 2) — an unprovable pair is
# never "clean" (a half-applied wave merge used to disarm the gate with a green `<none>`). The date is
# read from the declaration itself, so dated comments that merely mention the token cannot poison the
# read (review fix F3; the workflow's const-bump sed targets the same declaration shape).
#
# Removal guard (rm-176 acceptance: "catalog growth stays additive — no silent price removals"): any
# bundled model that would drop out of the keep-filter basis is reported as `retired` and sets
# removal_guard. The workflow refuses to auto-PR while removal_guard is set: a bundle model vanishing
# without an upstream deprecation trail is exactly the silent-unpricing event this gate exists to stop.
#
# Exit contract: 0 = no actionable drift and no desync; 1 = actionable drift (any class) or desync or
# removal guard; 2 = infra error (fetch/parse/missing input — including an unprovable desync pair:
# absent const declaration or absent _snapshot.date; fail-closed, never "clean"). Machine report: --json.
#
# Usage:
#   scripts/pricing/drift-check.sh                 # fetch live LiteLLM and compare against the bundle
#   scripts/pricing/drift-check.sh --live F.json   # offline: use a cached copy of the live catalog
#   ... --bundle F.json --pricing-rs F.rs          # overrides for red-arm testing
#   ... --json                                     # emit the machine report on stdout (summary goes to stderr
#                                                  #   then — stdout is PURE JSON; review-2 fix F2)
#   scripts/pricing/drift-check.sh --selftest      # seven embedded offline arms, rc 0 iff all pass

set -euo pipefail

url="https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$here/../.." && pwd)"
live=""
bundle="$repo_root/crates/agenttrace-core/src/pricing_snapshot.json"
pricing_rs="$repo_root/crates/agenttrace-core/src/pricing.rs"
emit_json=0
selftest=0

while [ $# -gt 0 ]; do
  case "$1" in
    --live) live="${2:?--live needs a file}"; shift 2 ;;
    --bundle) bundle="${2:?--bundle needs a file}"; shift 2 ;;
    --pricing-rs) pricing_rs="${2:?--pricing-rs needs a file}"; shift 2 ;;
    --json) emit_json=1; shift ;;
    --selftest) selftest=1; shift ;;
    -h|--help) sed -n '2,/^set -euo pipefail$/p' "$0" | sed '$d'; exit 0 ;;
    *) echo "drift-check: unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done

# analyze <live_json> <bundle_json> <const_date> <emit_json 0|1>
# Prints the human summary, optionally the JSON report, and returns the contract rc.
analyze() {
  python3 - "$1" "$2" "$3" "$4" <<'PYEOF'
import json, sys

live_path, bundle_path, const_date, emit_json = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4] == "1"

try:
    with open(live_path, "r", encoding="utf-8") as fh:
        live = json.load(fh)
    with open(bundle_path, "r", encoding="utf-8") as fh:
        bundle = json.load(fh)
except Exception as exc:  # infra: unreadable/malformed input
    print(f"drift-check: infra error reading inputs: {exc}", file=sys.stderr)
    sys.exit(2)

if not isinstance(live, dict) or not isinstance(bundle, dict):
    print("drift-check: infra error: catalogs must be JSON objects", file=sys.stderr)
    sys.exit(2)

# Keep-filter basis — must mirror scripts/pricing/update-snapshot.sh verbatim
# (rm-176 2026-10-10: image_generation models with per-token rates are part of
# the builder's keep set; step/resolution-qualified image keys stay out).
MODES_KEPT = ("chat", "image_generation")

def kept(catalog):
    out = {}
    for name, row in catalog.items():
        if name == "_snapshot" or name == "_plan_scope" or not isinstance(row, dict):
            continue
        if row.get("mode") not in MODES_KEPT:
            continue
        inp = float(row.get("input_cost_per_token") or 0.0)
        outp = float(row.get("output_cost_per_token") or 0.0)
        if inp > 0.0 or outp > 0.0:
            out[name] = row
    return out

# Field-comparison rule: drift is only meaningful on the fields the builder actually PINS
# (update-snapshot.sh writes input/output per-token costs, cache_creation/cache_read costs, mode,
# litellm_provider, max_input_tokens when int>0, deprecation_date when set). Comparing anything
# else (e.g. max_output_tokens, which the builder drops) produced 2,802 phantom diffs — diagnosed
# 2026-10-05 against the real bundle; see the cycle-1 record.
COST_FIELDS = ["input_cost_per_token", "output_cost_per_token",
               "cache_creation_input_token_cost", "cache_read_input_token_cost"]

try:
    live_kept, bundle_kept = kept(live), kept(bundle)
    meta = bundle.get("_snapshot") or {}
    bundle_date = meta.get("date")
    if not bundle_date:
        print("drift-check: infra error: bundle has no _snapshot.date stamp (fail-closed, review fix F1: "
              "an absent stamp is never 'clean' — reconcile the bundle)", file=sys.stderr)
        sys.exit(2)

    new_costed = sorted(set(live_kept) - set(bundle_kept))
    retired = sorted(set(bundle_kept) - set(live_kept))
    rate_changed, deprecation_new, context_changed, provider_changed = [], [], [], []
    for name in sorted(set(live_kept) & set(bundle_kept)):
        lrow, brow = live_kept[name], bundle_kept[name]
        if any(float(lrow.get(f) or 0.0) != float(brow.get(f) or 0.0) for f in COST_FIELDS):
            rate_changed.append(name)
        if lrow.get("deprecation_date") and lrow.get("deprecation_date") != brow.get("deprecation_date"):
            deprecation_new.append(name)
        if lrow.get("max_input_tokens") != brow.get("max_input_tokens"):
            context_changed.append(name)
        if (lrow.get("litellm_provider") or "") != (brow.get("litellm_provider") or ""):
            provider_changed.append(name)

    desync = const_date != bundle_date  # both sides proven present above (fail-closed, review fix F1)
    removal_guard = bool(retired)
    actionable = bool(new_costed or retired or rate_changed or deprecation_new or desync)
except SystemExit:
    raise
except Exception as exc:  # malformed catalog SHAPES (list-typed costs, _snapshot as a string, ...) are
    # INFRA failures (rc 2) per the exit contract — never actionable drift. Pre-fix (review-2 fix F3,
    # 2026-10-06): a crafted live row crashed kept() with a TypeError traceback and rc 1, mislabeling
    # a parse failure as drift and steering the workflow into bump mode with an empty report.
    print(f"drift-check: infra error: malformed catalog content ({type(exc).__name__}: {exc}) "
          "— fail-closed, never actionable drift (review-2 fix F3)", file=sys.stderr)
    sys.exit(2)

def sample(names):
    shown, extra = names[:10], len(names) - 10
    return ", ".join(shown) + (f" (+{extra} more)" if extra > 0 else "")

# review-2 fix F2: with --json the summary goes to STDERR so stdout is PURE JSON (json.tool/jq
# parseable); without --json the summary stays on stdout (the workflow's regen-verify grep for
# 'costed-chat models' reads a no---json run).
out = sys.stderr if emit_json else sys.stdout
print(f"pricing drift check (keep-filter basis: mode in (chat,image_generation) AND input-or-output per-token cost>0)", file=out)
print(f"  bundled: {len(bundle_kept)} costed-chat models, _snapshot.date={bundle_date or '<none>'}, PRICING_SNAPSHOT_DATE={const_date or '<none>'}", file=out)
print(f"  live   : {len(live_kept)} costed-chat models", file=out)
print(f"  new-costed: {len(new_costed)}{' — ' + sample(new_costed) if new_costed else ''}", file=out)
print(f"  retired   : {len(retired)}{' — ' + sample(retired) if retired else ''} (removal_guard={'SET' if removal_guard else 'clear'})", file=out)
print(f"  rate-changed: {len(rate_changed)}{' — ' + sample(rate_changed) if rate_changed else ''} (4 cost fields incl. cache)", file=out)
print(f"  deprecation-new: {len(deprecation_new)}{' — ' + sample(deprecation_new) if deprecation_new else ''}", file=out)
print(f"  context-window-changed (max_input only, informational): {len(context_changed)}{' — ' + sample(context_changed) if context_changed else ''}", file=out)
print(f"  provider-changed (informational): {len(provider_changed)}{' — ' + sample(provider_changed) if provider_changed else ''}", file=out)
if desync:
    print(f"  DESYNC (assess A1): pricing.rs const {const_date} != bundle _snapshot.date {bundle_date} — update-snapshot.sh's manual const bump was skipped", file=out)
if emit_json:
    report = {
        "basis": "mode in (chat,image_generation) AND (input_cost_per_token>0 OR output_cost_per_token>0)",
        "bundle": {"date": bundle_date, "costed_chat_models": len(bundle_kept)},
        "live": {"costed_chat_models": len(live_kept)},
        "pricing_snapshot_date_const": const_date,
        "counts": {
            "new_costed": len(new_costed), "retired": len(retired), "rate_changed": len(rate_changed),
            "deprecation_new": len(deprecation_new), "context_window_changed": len(context_changed),
            "provider_changed": len(provider_changed),
        },
        "examples": {"new_costed": new_costed[:10], "retired": retired[:10], "rate_changed": rate_changed[:10],
                     "deprecation_new": deprecation_new[:10], "context_window_changed": context_changed[:10],
                     "provider_changed": provider_changed[:10]},
        "desync": desync, "removal_guard": removal_guard, "actionable_drift": actionable,
    }
    print(json.dumps(report, indent=2, sort_keys=True))

sys.exit(1 if actionable else 0)
PYEOF
}

const_date_of() { # <pricing.rs path> -> the const's date, anchored on the DECLARATION `const PRICING_SNAPSHOT_DATE: &str = "<date>"` (survives upstream #313 root-slim renames; immune to dated comments that merely mention the token — review fix F3; same shape the workflow's const-bump sed targets)
  grep -oE "const[[:space:]]+PRICING_SNAPSHOT_DATE[[:space:]]*:[[:space:]]*&'?(static[[:space:]]+)?str[[:space:]]*=[[:space:]]*\"[0-9]{4}-[0-9]{2}-[0-9]{2}\"" "$1" 2>/dev/null \
    | grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}' | head -1 || true
}

run_check() { # resolves inputs, fetches live if needed, dispatches analyze; returns contract rc
  local tmp=""
  if [ -z "$live" ]; then
    tmp="$(mktemp -t drift-live.XXXXXX.json)"
    if ! curl -fsSL --max-time 60 "$url" -o "$tmp"; then
      echo "drift-check: infra error: fetch failed: $url" >&2
      rm -f "$tmp"
      return 2
    fi
    live="$tmp"
  fi
  for f in "$live" "$bundle"; do
    if [ ! -f "$f" ]; then echo "drift-check: infra error: missing input: $f" >&2; return 2; fi
  done
  if [ ! -f "$pricing_rs" ]; then
    echo "drift-check: infra error: pricing.rs not found: $pricing_rs" >&2
    return 2
  fi
  local rc=0 const_date
  const_date="$(const_date_of "$pricing_rs")"
  if [ -z "$const_date" ]; then
    echo "drift-check: infra error: no 'const PRICING_SNAPSHOT_DATE: &str = \"<date>\"' declaration found in $pricing_rs (fail-closed, review fix F1: an absent token is never 'clean' — reconcile pricing.rs)" >&2
    return 2
  fi
  analyze "$live" "$bundle" "$const_date" "$emit_json" || rc=$?
  [ -n "$tmp" ] && rm -f "$tmp"
  return "$rc"
}

self_test() { # seven embedded offline arms; rc 0 iff all pass (fixtures never touch the repo)
  local t rc ok=0
  t="$(mktemp -d -t drift-selftest.XXXXXX)"
  trap 'rm -rf "$t"' RETURN

  cat > "$t/base.json" <<'FIX'
{"model-a": {"mode": "chat", "input_cost_per_token": 3e-06, "output_cost_per_token": 1.5e-05, "max_input_tokens": 200000, "max_output_tokens": 8192},
 "model-b": {"mode": "chat", "input_cost_per_token": 1e-06, "output_cost_per_token": 4e-06, "max_input_tokens": 128000, "max_output_tokens": 4096},
 "model-d": {"mode": "chat", "input_cost_per_token": 5e-07, "output_cost_per_token": 1e-06},
 "not-chat": {"mode": "embedding", "input_cost_per_token": 1e-07, "output_cost_per_token": 0},
 "chat-uncosted": {"mode": "chat", "input_cost_per_token": 0, "output_cost_per_token": 0},
 "_snapshot": {"date": "2026-10-04", "models": 3, "source": "BerriAI/litellm"}}
FIX
  cat > "$t/live-parity.json" <<'FIX'
{"model-a": {"mode": "chat", "input_cost_per_token": 3e-06, "output_cost_per_token": 1.5e-05, "max_input_tokens": 200000, "max_output_tokens": 8192},
 "model-b": {"mode": "chat", "input_cost_per_token": 1e-06, "output_cost_per_token": 4e-06, "max_input_tokens": 128000, "max_output_tokens": 4096},
 "model-d": {"mode": "chat", "input_cost_per_token": 5e-07, "output_cost_per_token": 1e-06},
 "not-chat": {"mode": "embedding", "input_cost_per_token": 1e-07, "output_cost_per_token": 0},
 "chat-uncosted": {"mode": "chat", "input_cost_per_token": 0, "output_cost_per_token": 0}}
FIX
  printf 'const X: &str = PRICING_SNAPSHOT_DATE;\nconst PRICING_SNAPSHOT_DATE: &str = "2026-10-04";\n' > "$t/pricing.rs"

  # Arm 1 — parity (bundle == live on the keep-filter basis, const == meta): expect rc 0.
  rc=0; "$0" --live "$t/live-parity.json" --bundle "$t/base.json" --pricing-rs "$t/pricing.rs" > "$t/out1.txt" 2>&1 || rc=$?
  if [ "$rc" -eq 0 ]; then echo "  arm 1 parity ............ PASS (rc 0)"; else echo "  arm 1 parity ............ FAIL (rc $rc)"; sed -n '1,8p' "$t/out1.txt"; ok=1; fi

  # Arm 2 — actionable drift: new-costed + rate change + new deprecation, no removals: expect rc 1, guard clear.
  python3 - "$t" <<'PY'
import json, sys
t = sys.argv[1]
live = json.load(open(f"{t}/live-parity.json"))
live["model-c"] = {"mode": "chat", "input_cost_per_token": 2e-06, "output_cost_per_token": 8e-06}
live["model-a"]["output_cost_per_token"] = 2.0e-05
live["model-b"]["deprecation_date"] = "2026-11-01"
json.dump(live, open(f"{t}/live-drift.json", "w"))
PY
  rc=0; "$0" --live "$t/live-drift.json" --bundle "$t/base.json" --pricing-rs "$t/pricing.rs" --json > "$t/out2.txt" 2>&1 || rc=$?
  if [ "$rc" -eq 1 ] && grep -q '"new_costed": 1' "$t/out2.txt" && grep -q '"rate_changed": 1' "$t/out2.txt" \
     && grep -q '"deprecation_new": 1' "$t/out2.txt" && grep -q '"retired": 0' "$t/out2.txt" \
     && grep -q '"removal_guard": false' "$t/out2.txt"; then
    echo "  arm 2 drift classes ..... PASS (rc 1, new=1 rate=1 deprecation=1 retired=0, guard clear)"
  else echo "  arm 2 drift classes ..... FAIL (rc $rc)"; sed -n '1,12p' "$t/out2.txt"; ok=1; fi

  # Arm 3 — desync (assess A1): const date disagrees with _snapshot.date: expect rc 1 with the DESYNC line.
  printf 'const PRICING_SNAPSHOT_DATE: &str = "2026-09-30";\n' > "$t/pricing-desync.rs"
  rc=0; "$0" --live "$t/live-parity.json" --bundle "$t/base.json" --pricing-rs "$t/pricing-desync.rs" > "$t/out3.txt" 2>&1 || rc=$?
  if [ "$rc" -eq 1 ] && grep -q 'DESYNC (assess A1)' "$t/out3.txt"; then
    echo "  arm 3 desync gate ....... PASS (rc 1, DESYNC surfaced)"
  else echo "  arm 3 desync gate ....... FAIL (rc $rc)"; sed -n '1,8p' "$t/out3.txt"; ok=1; fi

  # Arm 4 — removal protection: a bundled model absent from live: expect rc 1 with removal_guard SET.
  python3 - "$t" <<'PY'
import json, sys
t = sys.argv[1]
live = json.load(open(f"{t}/live-parity.json"))
del live["model-d"]
json.dump(live, open(f"{t}/live-removal.json", "w"))
PY
  rc=0; "$0" --live "$t/live-removal.json" --bundle "$t/base.json" --pricing-rs "$t/pricing.rs" --json > "$t/out4.txt" 2>&1 || rc=$?
  if [ "$rc" -eq 1 ] && grep -q '"retired": 1' "$t/out4.txt" && grep -q '"removal_guard": true' "$t/out4.txt"; then
    echo "  arm 4 removal guard ..... PASS (rc 1, retired=1, removal_guard SET)"
  else echo "  arm 4 removal guard ..... FAIL (rc $rc)"; sed -n '1,12p' "$t/out4.txt"; ok=1; fi

  # Arm 5 — fail-closed on absent proof inputs (review fix F1): a pricing.rs without the
  # declaration, and a bundle without _snapshot.date, are INFRA failures (rc 2) — never "clean".
  # (Pre-fix behavior, proven by the review's red arms: green rc 0 with `<none>` — the fail-open hole.)
  printf 'const OTHER_DATE: &str = "2026-10-04";\n' > "$t/pricing-noconst.rs"
  local a=0 b=0
  rc=0; "$0" --live "$t/live-parity.json" --bundle "$t/base.json" --pricing-rs "$t/pricing-noconst.rs" > "$t/out5a.txt" 2>&1 || rc=$?
  [ "$rc" -eq 2 ] && grep -q 'infra error.*PRICING_SNAPSHOT_DATE.*declaration' "$t/out5a.txt" && a=1
  python3 - "$t" <<'PY'
import json, sys
t = sys.argv[1]
bundle = json.load(open(f"{t}/base.json"))
del bundle["_snapshot"]
json.dump(bundle, open(f"{t}/bundle-nometa.json", "w"))
PY
  rc=0; "$0" --live "$t/live-parity.json" --bundle "$t/bundle-nometa.json" --pricing-rs "$t/pricing.rs" > "$t/out5b.txt" 2>&1 || rc=$?
  [ "$rc" -eq 2 ] && grep -q 'infra error.*_snapshot.date' "$t/out5b.txt" && b=1
  if [ "$a" -eq 1 ] && [ "$b" -eq 1 ]; then
    echo "  arm 5 fail-closed ....... PASS (absent const -> rc 2; absent _snapshot.date -> rc 2)"
  else echo "  arm 5 fail-closed ....... FAIL (const sub-arm a=$a, meta sub-arm b=$b)"; sed -n '1,6p' "$t/out5a.txt"; sed -n '1,6p' "$t/out5b.txt"; ok=1; fi

  # Arm 6 — declaration anchoring (review fix F3): a dated comment mentioning the token above
  # the real const must NOT poison the date read (pre-fix first-match logic produced a FALSE
  # desync rc 1 here, blocking legitimate auto-PRs).
  printf '/// snapshot was 2026-09-01 (old); keep PRICING_SNAPSHOT_DATE in sync with the bundle\nconst PRICING_SNAPSHOT_DATE: &str = "2026-10-04";\n' > "$t/pricing-poison.rs"
  rc=0; "$0" --live "$t/live-parity.json" --bundle "$t/base.json" --pricing-rs "$t/pricing-poison.rs" > "$t/out6.txt" 2>&1 || rc=$?
  if [ "$rc" -eq 0 ] && grep -q 'PRICING_SNAPSHOT_DATE=2026-10-04' "$t/out6.txt" && ! grep -q 'DESYNC' "$t/out6.txt"; then
    echo "  arm 6 const anchor ...... PASS (dated doc comment ignored, read 2026-10-04, rc 0)"
  else echo "  arm 6 const anchor ...... FAIL (rc $rc)"; sed -n '1,8p' "$t/out6.txt"; ok=1; fi

  # Arm 7 — malformed catalog shapes are INFRA failures (review-2 fix F3): a live row with a
  # list-typed cost, and a bundle whose _snapshot is a string, must exit rc 2 with the infra
  # message on stderr and EMPTY stdout (pre-fix: TypeError/AttributeError traceback at rc 1 —
  # a parse failure mislabeled as actionable drift, steering the workflow into bump mode
  # with an empty drift report).
  python3 - "$t" <<'PY'
import json, sys
t = sys.argv[1]
json.dump({"m": {"mode": "chat", "input_cost_per_token": [3e-06], "output_cost_per_token": 1.5e-05}},
          open(f"{t}/live-badtype.json", "w"))
json.dump({"_snapshot": "2026-10-04", "m": {"mode": "chat", "input_cost_per_token": 3e-06, "output_cost_per_token": 1.5e-05}},
          open(f"{t}/bundle-badmeta.json", "w"))
PY
  local c=0 d=0
  rc=0; "$0" --live "$t/live-badtype.json" --bundle "$t/base.json" --pricing-rs "$t/pricing.rs" > "$t/out7a.txt" 2> "$t/out7a.err" || rc=$?
  [ "$rc" -eq 2 ] && [ ! -s "$t/out7a.txt" ] && grep -q 'infra error: malformed catalog content' "$t/out7a.err" && c=1
  rc=0; "$0" --live "$t/live-parity.json" --bundle "$t/bundle-badmeta.json" --pricing-rs "$t/pricing.rs" > "$t/out7b.txt" 2> "$t/out7b.err" || rc=$?
  [ "$rc" -eq 2 ] && [ ! -s "$t/out7b.txt" ] && grep -q 'infra error: malformed catalog content' "$t/out7b.err" && d=1
  if [ "$c" -eq 1 ] && [ "$d" -eq 1 ]; then
    echo "  arm 7 malformed shapes ... PASS (list-typed cost -> rc 2; _snapshot string -> rc 2; stdout empty)"
  else echo "  arm 7 malformed shapes ... FAIL (badtype sub-arm c=$c, badmeta sub-arm d=$d)"; sed -n '1,6p' "$t/out7a.txt"; sed -n '1,6p' "$t/out7a.err"; ok=1; fi

  # Control — the keep-filter basis itself: non-chat and uncosted rows never count either side.
  if grep -q 'costed-chat models' "$t/out1.txt"; then
    echo "  control keep-filter ...... PASS (basis line present)"
  else echo "  control keep-filter ...... FAIL"; ok=1; fi

  return "$ok"
}

if [ "$selftest" -eq 1 ]; then
  echo "drift-check --selftest (seven offline arms, no network)"
  self_test
  rc=$?
  [ "$rc" -eq 0 ] && echo "selftest: ALL ARMS PASS" || echo "selftest: FAILURES PRESENT"
  exit "$rc"
fi

run_check
