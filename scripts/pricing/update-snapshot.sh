#!/usr/bin/env bash
# Regenerates crates/agenttrace-core/src/pricing_snapshot.json — the vendored
# offline pricing catalog. Requires network. Run manually, then commit the
# result and update PRICING_SNAPSHOT_DATE in crates/agenttrace-core/src/pricing.rs
# to match the printed date.
set -euo pipefail

url="https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json"
out="crates/agenttrace-core/src/pricing_snapshot.json"
date="$(date -u +%F)"
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

curl -fsSL "$url" -o "$tmp"

python3 - "$tmp" "$out" "$date" <<'EOF'
import json
import sys

src, dst, date = sys.argv[1], sys.argv[2], sys.argv[3]
data = json.load(open(src))
keep = {}
for key, value in data.items():
    if not isinstance(value, dict):
        continue
    if value.get("mode") != "chat":
        continue
    inp = value.get("input_cost_per_token") or 0
    outp = value.get("output_cost_per_token") or 0
    if inp == 0 and outp == 0:
        continue
    keep[key] = {
        "input_cost_per_token": inp,
        "output_cost_per_token": outp,
        "cache_creation_input_token_cost": value.get("cache_creation_input_token_cost") or 0,
        "cache_read_input_token_cost": value.get("cache_read_input_token_cost") or 0,
        "mode": "chat",
        "litellm_provider": value.get("litellm_provider") or "",
    }
    # Vendor context window and deprecation date ride along when the
    # source carries them (rm-231 / rm-419). The source file is literally
    # named model_prices_and_context_WINDOW.json — the context-window
    # field used to be the one thing this trim dropped. Zero/empty
    # values are omitted so "carries a window" stays greppable.
    max_input = value.get("max_input_tokens")
    if isinstance(max_input, int) and max_input > 0:
        keep[key]["max_input_tokens"] = max_input
    deprecation = value.get("deprecation_date")
    if isinstance(deprecation, str) and deprecation.strip():
        keep[key]["deprecation_date"] = deprecation.strip()
snapshot = {
    "_snapshot": {
        "source": "BerriAI/litellm model_prices_and_context_window.json",
        "date": date,
        "models": len(keep),
    }
}
snapshot.update(keep)
with open(dst, "w") as f:
    json.dump(snapshot, f, separators=(",", ":"), sort_keys=True)
    f.write("\n")
print(f"wrote {len(keep)} chat models to {dst} (snapshot date {date})")
EOF
