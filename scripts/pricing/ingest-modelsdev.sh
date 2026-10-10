#!/usr/bin/env bash
# rm-845: ingest subscription plan-tier entries from models.dev into the
# bundled snapshot's `_plan_scope` section (add/update ONLY — never drops
# existing plan entries; the LiteLLM model section is untouched).
#
# Authority: models.dev api.json coding-plan providers (zai-coding-plan,
# alibaba-token-plan, zai direct). Provider-id grammar cross-checked
# against ccusage#1832 ("recognize current Z.ai built-in plan provider
# IDs", merged 2026-10-08).
#
# Usage: bash scripts/pricing/ingest-modelsdev.sh [api.json path]
#   (fetches https://models.dev/api.json when no path is given)
set -euo pipefail

SNAPSHOT="$(dirname "$0")/../../crates/agenttrace-core/src/pricing_snapshot.json"
API="${1:-}"
if [[ -z "$API" ]]; then
  API="$(mktemp /tmp/modelsdev-XXXXXX.json)"
  trap 'rm -f "$API"' EXIT
  curl -fsSL --max-time 60 "https://models.dev/api.json" -o "$API"
fi

python3 - "$API" "$SNAPSHOT" <<'PY'
import json, sys, collections

api_path, snapshot_path = sys.argv[1], sys.argv[2]
with open(api_path) as fh:
    api = json.load(fh)
with open(snapshot_path) as fh:
    snapshot = json.load(fh, object_pairs_hook=collections.OrderedDict)

# Plan-scope providers: subscription coding plans (per-token included ->
# 0/0) and zai direct (real per-Mtok rates). Extend deliberately.
PLAN_PROVIDERS = ("zai-coding-plan", "alibaba-token-plan")
DIRECT_PROVIDERS = ("zai",)

wanted = collections.OrderedDict()
for provider in PLAN_PROVIDERS:
    models = api.get(provider, {}).get("models", {})
    for model_id, model in models.items():
        wanted[f"{provider}/{model_id}"] = collections.OrderedDict([
            ("input_per_mtok", 0.0),
            ("output_per_mtok", 0.0),
            ("note", f"{provider} subscription: included (no per-token charge)"),
        ])
for provider in DIRECT_PROVIDERS:
    models = api.get(provider, {}).get("models", {})
    for model_id, model in models.items():
        cost = model.get("cost") or {}
        # models.dev rates are per MILLION tokens.
        wanted[f"{provider}/{model_id}"] = collections.OrderedDict([
            ("input_per_mtok", cost.get("input") or 0.0),
            ("output_per_mtok", cost.get("output") or 0.0),
            ("note", f"{provider} direct API per-token rate (models.dev)"),
        ])

existing = snapshot.get("_plan_scope", collections.OrderedDict())
plan = collections.OrderedDict()
added = updated = kept = 0
for key, entry in wanted.items():
    if key not in existing:
        added += 1
        plan[key] = entry
    # dict() compare: OrderedDict equality is ORDER-sensitive, and the
    # on-disk form is always key-sorted (json.dumps sort_keys) while the
    # constructor emits (input, output, note) — an order-only compare
    # would count every content-equal entry as "updated" on every run
    # (bytes stay stable because the dump re-sorts; the counter lied).
    elif dict(existing[key]) != dict(entry):
        updated += 1
        plan[key] = entry
    else:
        kept += 1
        plan[key] = existing[key]
dropped = [k for k in existing if k != "provenance" and k not in wanted]
for key in dropped:  # add/update-only ingester: keep dropped keys too
    plan[key] = existing[key]
plan["provenance"] = collections.OrderedDict([
    ("source", "models.dev api.json"),
    ("fetched", __import__("datetime").datetime.now(__import__("datetime").timezone.utc).date().isoformat()),
    ("authority", "ccusage#1832 provider-id grammar; rm-845"),
])

snapshot["_plan_scope"] = plan
with open(snapshot_path, "w") as fh:
    fh.write(json.dumps(snapshot, sort_keys=True, separators=(",", ":")) + "\n")
print(f"_plan_scope: +{added} added, {updated} updated, {kept} unchanged, "
      f"{len(dropped)} retained-by-policy, {len(plan)-1} total entries")
PY
