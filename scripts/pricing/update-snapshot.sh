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
# rm-1440 change-class ledger: every successful refresh appends one JSON line
# to scripts/pricing/snapshot-change-ledger.jsonl recording the diff against
# the committed bundle, classified — growth (new priced key), correction
# (rate/flex mutation on a retained key), window (context-window-only
# mutation), removal (key drop through a dated manual-drop note),
# field_removal (a cost/context field the live source dropped for a retained
# key — RESTORED from the prior bundle, never silently absorbed or zeroed),
# and oscillation (a cell moving back to a value it already held in a recent
# refresh — QUARANTINED: the prior value is retained and the attempt is
# disclosed, because research run b9963213 proved upstream corrections
# oscillate within hours and "latest wins" can regress a corrected cell).
# The same histogram is printed on every refresh and is replayable offline
# via --histogram (the pricing-drift PR body embeds it).
#
# Run from anywhere (paths resolve from this script's location); after a
# refresh commit the result, update PRICING_SNAPSHOT_DATE in
# crates/agenttrace-core/src/pricing.rs to match the printed date, and append
# the printed added/removed delta to the rm-176 row in ROADMAP.md (rm-588
# acceptance clause 3).
#
# Arguments (rm-901 guard): no-args runs the refresh; --help/-h prints
# usage; --check reports upstream-vs-vendored drift WITHOUT writing anything
# (delegates to drift-check.sh; exit codes follow it: 0 clean, 1 actionable,
# 2 infra); --histogram replays the last ledger line read-only; --selftest
# runs the offline fixture arms (scripts/pricing/fixtures/change-class/) in a
# temp directory — no network, no tracked writes. Exactly ONE argument is
# accepted: any unknown flag AND any second argument after a recognized one
# are REFUSED (rc 2), so a stray invocation can never rewrite the tracked
# bundle; an unreadable LITELLM_SNAPSHOT_SRC is infra rc 2 (review fix F4/
# F7), never a silent set -e rc 1.
#
# Environment overrides:
#   LITELLM_SNAPSHOT_SRC  path to a captured LiteLLM catalog file; refreshes
#                         (and --check) offline from it instead of fetching
#                         (deterministic re-runs; verify the capture is
#                         recent enough first)
#   SNAPSHOT_DATE         date stamped into _snapshot.date and compared
#                         against manual-drop notes (default: today UTC)
set -euo pipefail

url="https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$here/../.." && pwd)"
out="$repo_root/crates/agenttrace-core/src/pricing_snapshot.json"
drops="$repo_root/scripts/pricing/snapshot-drops.txt"
ledger="$here/snapshot-change-ledger.jsonl"
fixtures="$here/fixtures/change-class"
date="${SNAPSHOT_DATE:-$(date -u +%F)}"
src="${LITELLM_SNAPSHOT_SRC:-}"

usage() {
    cat <<'USAGE'
Regenerates crates/agenttrace-core/src/pricing_snapshot.json (the vendored
offline pricing catalog) through the rm-588 never-drop union contract.
Live-source keys containing '*' (glob rows) are skipped (rm-903): the
offline lookup is exact-key and can never match them.

Usage:
    scripts/pricing/update-snapshot.sh             run the refresh (no arguments)
    scripts/pricing/update-snapshot.sh --check     report upstream-vs-vendored drift,
                                                   WRITING NOTHING (drift-check.sh
                                                   semantics: rc 0 clean / 1
                                                   actionable / 2 infra)
    scripts/pricing/update-snapshot.sh --histogram replay the last rm-1440
                                                   change-class ledger line (read-only)
    scripts/pricing/update-snapshot.sh --selftest  offline fixture arms: guards,
                                                   ledger, field-removal restore
                                                   (incl. provider), oscillation
                                                   quarantine, removal refusal,
                                                   event-cap ordering — no network,
                                                   no writes
    scripts/pricing/update-snapshot.sh --help      print this help

Any other argument is REFUSED — and exactly one argument is accepted: a
second argument after a recognized flag is REFUSED (rc 2) too. The refresh
must never be triggered by a stray invocation (accidental --help, a
tab-completed flag, a typo), because it rewrites a tracked file. An
unreadable LITELLM_SNAPSHOT_SRC is an infra failure (rc 2), never a silent
rc 1.

Every successful refresh appends a change-class ledger line (rm-1440) to
scripts/pricing/snapshot-change-ledger.jsonl: growth / correction / window /
removal / field_removal (restored from the prior bundle, never silently
zeroed) / oscillation (quarantined — prior value retained, attempt
disclosed).

Environment overrides:
    LITELLM_SNAPSHOT_SRC  path to a captured LiteLLM catalog file; refreshes
                          (and --check) offline from it instead of fetching
    SNAPSHOT_DATE         date stamped into _snapshot.date and compared
                          against manual-drop notes (default: today UTC)

After a refresh: update PRICING_SNAPSHOT_DATE in pricing.rs, append the
printed added/removed delta to the rm-176 row in ROADMAP.md (rm-588 clause 3),
and commit the result together with the new ledger line.
USAGE
}

# Shared python core (rm-1440): one program, two entry modes — `refresh`
# (argv: src dst date drops ledger) and `selftest` (argv: fixtures tmpdir).
core_py="$(cat <<'PYCORE'
import hashlib
import json
import os
import shutil
import sys

RATE_FIELDS = (
    "input_cost_per_token",
    "output_cost_per_token",
    "cache_creation_input_token_cost",
    "cache_read_input_token_cost",
)
FLEX_FIELDS = (
    "input_cost_per_token_flex",
    "output_cost_per_token_flex",
    "cache_creation_input_token_cost_flex",
    "cache_read_input_token_cost_flex",
)
MUTATION_FIELDS = RATE_FIELDS + FLEX_FIELDS + ("max_input_tokens",)
# Fields a retained key may legitimately lose upstream: every meaningful
# one is RESTORED from the prior bundle (rm-1440: never silently absorbed —
# trim() stamps absent cache rates as 0, which used to zero a live price).
RESTORE_FIELDS = MUTATION_FIELDS + ("deprecation_date", "litellm_provider")

LEDGER_EVENT_CAP = 400
LEDGER_HISTORY_LINES = 20


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
        }
        # rm-1440 review fix F5: the provider rides along ONLY when the live
        # source carries a non-empty one — never stamped "". Stamping "" made
        # the field_removal gone-check (lv is None) unreachable for this field
        # (dead restore arm), so an upstream provider-field loss landed
        # silently as "". Omission is safe downstream: pricing.rs loads
        # litellm_provider with #[serde(default)] (pricing.rs:116), and a
        # retained key losing the field upstream is RESTORED from the prior
        # bundle + recorded as a field_removal event like every other
        # RESTORE_FIELD.
        provider = value.get("litellm_provider")
        if isinstance(provider, str) and provider.strip():
            entry["litellm_provider"] = provider.strip()
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
        for flex_field in FLEX_FIELDS:
            flex = value.get(flex_field)
            if isinstance(flex, (int, float)) and flex > 0:
                entry[flex_field] = flex
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
            print(f"drop note not yet effective (dated {when}, snapshot {today}): {key}")
            continue
        drops[key] = (when, reason)
    return drops


def load_history(path):
    # rm-1440 oscillation window: the values each (key, field) has held over
    # the last LEDGER_HISTORY_LINES refreshes, from recorded change events.
    # A refresh moving a cell BACK to one of those values is treated as an
    # upstream correction oscillation, not a new truth (research run
    # b9963213 N1: the same cell moved 0.3 -> 0.0369 -> 0.0252 within ~2h).
    hist = {}
    try:
        with open(path) as f:
            lines = f.read().splitlines()[-LEDGER_HISTORY_LINES:]
    except FileNotFoundError:
        return hist
    for line in lines:
        try:
            rec = json.loads(line)
        except ValueError:
            continue
        for ev in rec.get("events", []):
            if ev.get("class") in ("correction", "window", "oscillation"):
                cell = (ev.get("key"), ev.get("field"))
                for v in (ev.get("old"), ev.get("new")):
                    if v is not None:
                        hist.setdefault(cell, []).append(v)
    return hist


def refresh(src, dst, date, drops_path, ledger_path, quiet=False):
    live = trim(json.load(open(src)))
    prior_doc = json.load(open(dst))
    prior = {k: v for k, v in prior_doc.items() if k != "_snapshot"}

    # Vacuous-refresh floor (review hardening on rm-588): a degenerate live
    # source — valid JSON that prices nothing, or an implausibly small share
    # of the prior catalog — must REFUSE instead of stamping a fresh
    # _snapshot.date on 100% stale prior content carried by the union merge.
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

    # rm-1440 field-removal restore (BEFORE the mutation scan, so a restored
    # field is not also counted as a mutation): a retained key losing a
    # priced/context field upstream keeps the prior value and the loss is
    # recorded as a field_removal event — never silently absorbed, never
    # silently zeroed through trim()'s `or 0` stamping.
    field_removals = []
    for key in sorted(set(live) & set(prior)):
        for field in RESTORE_FIELDS:
            pv = prior[key].get(field)
            if pv is None or pv == 0 or pv == "":
                continue
            lv = live[key].get(field)
            if field in ("cache_creation_input_token_cost", "cache_read_input_token_cost"):
                gone = lv == 0
            else:
                gone = lv is None
            if gone:
                live[key][field] = pv
                field_removals.append((key, field, pv))

    # rm-1440 mutation scan with the oscillation quarantine. Rate/flex
    # changes are corrections, window-only changes are windows; a change
    # whose new value the cell already held within the ledger window is an
    # oscillation — quarantined: the prior value is retained, the attempt is
    # recorded and disclosed. A persistent oscillation re-records every
    # refresh (visible, auditable) until an operator reconciles it through
    # snapshot-drops.txt or a deliberate manual edit.
    hist = load_history(ledger_path)
    corrections = []
    windows = []
    oscillations = []
    for key in sorted(set(live) & set(prior)):
        for field in MUTATION_FIELDS:
            old = prior[key].get(field)
            new = live[key].get(field)
            if old == new:
                continue
            if new is not None and new in hist.get((key, field), []):
                live[key][field] = old
                oscillations.append((key, field, old, new))
            elif field == "max_input_tokens":
                windows.append((key, field, old, new))
            else:
                corrections.append((key, field, old, new))

    # Union merge (rm-588 arm 1): start from the trimmed live catalog, then
    # carry every previously priced key live no longer provides as-is.
    merged = dict(live)
    for key in sorted(prior):
        if key not in merged:
            merged[key] = prior[key]

    drops = manual_drops(drops_path, date)
    honored = []
    for key, (when, reason) in sorted(drops.items()):
        if key not in merged:
            if not quiet:
                print(f"drop note references a key not in the merge (inert): {key}")
            continue
        if key in live:
            if not quiet:
                print(f"drop note for a key still priced live (ignored this refresh): {key}")
            continue
        del merged[key]
        honored.append(key)
        if not quiet:
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
    floor = len(prior) - len(honored)
    if len(merged) < floor:
        sys.exit(
            f"REFUSED: post-refresh priced-model count {len(merged)} shrank below the "
            f"union floor {floor} (pre-refresh {len(prior)}, honored drops {len(honored)})"
        )

    retained = sorted(k for k in prior if k not in live and k in merged)
    adds = sorted(set(live) - set(prior))

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

    # rm-1440 ledger append: AFTER the bundle write, so a refused refresh
    # records nothing and a failed write cannot claim a refresh happened.
    # Event assembly order is load-bearing (rm-1440 review fix F6): the cap
    # below keeps the FIRST LEDGER_EVENT_CAP events, so the safety-critical
    # classes — removal (the only priced-key exit), field_removal (silent-
    # loss guard), oscillation (quarantine audit) — are assembled FIRST; a
    # >400-event refresh truncates the high-volume growth/correction/window
    # tails, never the classes an operator must be able to see. counts above
    # are computed from the full lists and stay complete under the cap.
    events = []
    for k in honored:
        events.append({"class": "removal", "key": k, "field": None, "old": None, "new": None})
    for (k, f, pv) in field_removals:
        events.append({"class": "field_removal", "key": k, "field": f, "old": pv, "new": None})
    for (k, f, old, new) in oscillations:
        events.append({"class": "oscillation", "key": k, "field": f, "old": old, "new": new})
    for k in adds:
        events.append({"class": "growth", "key": k, "field": None, "old": None, "new": None})
    for (k, f, old, new) in corrections:
        events.append({"class": "correction", "key": k, "field": f, "old": old, "new": new})
    for (k, f, old, new) in windows:
        events.append({"class": "window", "key": k, "field": f, "old": old, "new": new})
    truncated = 0
    if len(events) > LEDGER_EVENT_CAP:
        truncated = len(events) - LEDGER_EVENT_CAP
        events = events[:LEDGER_EVENT_CAP]
    line = {
        "date": date,
        "source_md5": hashlib.md5(open(src, "rb").read()).hexdigest(),
        "prior_models": len(prior),
        "live_models": len(live),
        "merged_models": len(merged),
        "counts": {
            "growth": len(adds),
            "correction": len(corrections),
            "window": len(windows),
            "removal": len(honored),
            "field_removal": len(field_removals),
            "oscillation": len(oscillations),
            "retained": len(retained),
        },
        "events": events,
    }
    if truncated:
        line["truncated_events"] = truncated
    with open(ledger_path, "a") as f:
        f.write(json.dumps(line, separators=(",", ":"), sort_keys=True) + "\n")

    if not quiet:
        print(
            f"wrote {len(merged)} chat models to {dst} (snapshot date {date}): "
            f"{len(prior)} pre-refresh | +{len(adds)} added | {len(corrections)} rate-mutated | "
            f"{len(windows)} window-only-changed | {len(retained)} retained-by-union | "
            f"{len(honored)} manual drop(s)"
        )
        print(
            f"change classes (rm-1440): growth=+{len(adds)} correction={len(corrections)} "
            f"window={len(windows)} removal={len(honored)} field_removal={len(field_removals)} "
            f"oscillation={len(oscillations)} quarantined | ledger: {len(events)} event(s) -> {ledger_path}"
        )
        if field_removals:
            shown = ", ".join(f"{k}.{f}" for k, f, _ in field_removals[:10])
            print(f"field removal restored from prior bundle (never silently absorbed): {shown}")
        if oscillations:
            shown = ", ".join(f"{k}.{f}" for k, f, _, _ in oscillations[:10])
            print(f"oscillation quarantined (prior value retained): {shown}")
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


def selftest(fixtures, tmp):
    def fail(msg):
        print(f"selftest: FAIL {msg}")
        sys.exit(1)

    def passed(msg):
        print(f"selftest: PASS {msg}")

    def run(src, dst, date, drops_path, ledger_path, want_ok):
        rc = 0
        try:
            refresh(src, dst, date, drops_path, ledger_path, quiet=True)
        except SystemExit as exc:
            rc = exc.code if isinstance(exc.code, int) else 1
        if want_ok and rc != 0:
            fail(f"refresh refused unexpectedly (rc={rc}) src={src}")
        if (not want_ok) and rc == 0:
            fail(f"refresh should have refused src={src}")

    def last_line(path):
        return json.loads(open(path).read().splitlines()[-1])

    base = f"{fixtures}/base.json"
    corr = f"{fixtures}/live-correction-only.json"
    fieldrm = f"{fixtures}/live-field-removal.json"
    osc = f"{fixtures}/live-osc-step2.json"
    drop = f"{fixtures}/live-drop-beta.json"
    empty_drops = f"{tmp}/empty-drops.txt"
    open(empty_drops, "w").close()
    day = "2026-10-14"

    # Arm 1 — correction-only window (identical key counts on both sides)
    # still refreshes and is classed as correction, not treated as a no-op
    # (research N1: md5+keycount gates cannot see these windows).
    a = f"{tmp}/a.json"
    led1 = f"{tmp}/led1.jsonl"
    shutil.copyfile(base, a)
    run(corr, a, day, empty_drops, led1, True)
    got = json.load(open(a))
    if got["prov/alpha"]["input_cost_per_token"] != 2.5e-06:
        fail("arm1: correction was not applied")
    if got["_snapshot"]["date"] != day:
        fail("arm1: snapshot date not stamped")
    c1 = last_line(led1)["counts"]
    if c1["correction"] != 1 or c1["growth"] != 0 or c1["oscillation"] != 0:
        fail(f"arm1: wrong ledger counts {c1}")
    passed("arm1 correction-only window refreshes and is classed (correction=1)")

    # Arm 2 — oscillation A->B->A': the second refresh moving the cell back
    # to a value it already held is QUARANTINED (prior value retained,
    # attempt disclosed), not applied.
    run(osc, a, day, empty_drops, led1, True)
    got = json.load(open(a))
    if got["prov/alpha"]["input_cost_per_token"] != 2.5e-06:
        fail("arm2: oscillation was applied (bundle regressed to the old value)")
    c2 = last_line(led1)["counts"]
    if c2["oscillation"] != 1 or c2["correction"] != 0:
        fail(f"arm2: wrong ledger counts {c2}")
    ev = [e for e in last_line(led1)["events"] if e["class"] == "oscillation"]
    if not ev or ev[0]["key"] != "prov/alpha" or ev[0]["old"] != 2.5e-06 or ev[0]["new"] != 3e-06:
        fail(f"arm2: oscillation event not recorded truthfully: {ev}")
    passed("arm2 oscillation quarantined, prior value retained, event recorded")

    # Arm 3 — field removal: a retained key losing cache_read / flex /
    # max_input / deprecation upstream keeps the prior values (RESTORED),
    # emits field_removal events, and is NEVER silently zeroed.
    b = f"{tmp}/b.json"
    led2 = f"{tmp}/led2.jsonl"
    shutil.copyfile(base, b)
    run(fieldrm, b, day, empty_drops, led2, True)
    beta = json.load(open(b))["prov/beta"]
    if beta["cache_read_input_token_cost"] != 4e-07:
        fail(f"arm3: cache_read silently zeroed: {beta.get('cache_read_input_token_cost')}")
    if "input_cost_per_token_flex" not in beta or beta["input_cost_per_token_flex"] != 5e-07:
        fail("arm3: flex rate silently lost")
    if beta.get("max_input_tokens") != 100000 or beta.get("deprecation_date") != "2027-01-01":
        fail("arm3: window/deprecation silently lost")
    if beta.get("litellm_provider") != "prov":
        fail(f"arm3: provider field silently lost (F5 restore arm): {beta.get('litellm_provider')!r}")
    c3 = last_line(led2)["counts"]
    if c3["field_removal"] != 5 or c3["correction"] != 0:
        fail(f"arm3: wrong ledger counts {c3}")
    passed("arm3 field removal restored (cache_read/flex/window/deprecation/provider), zero silent losses")

    # Arm 4 — never-drop union retention: an upstream key removal WITHOUT a
    # dated note must NOT lose the key — the union carries it, the ledger
    # classes it retained, and the refresh succeeds (the bundle can never
    # silently shed a priced key; the only exit is an honored note, arm 5).
    c = f"{tmp}/c.json"
    led3 = f"{tmp}/led3.jsonl"
    shutil.copyfile(base, c)
    before = open(c, "rb").read()
    run(drop, c, day, empty_drops, led3, True)
    got = json.load(open(c))
    if "prov/beta" not in got:
        fail("arm4: unnoted upstream removal lost a priced key (never-drop union broken)")
    if open(c, "rb").read() == before:
        fail("arm4: refresh did not rewrite the bundle (date not restamped)")
    c4 = last_line(led3)["counts"]
    if c4["retained"] != 1 or c4["removal"] != 0 or c4["correction"] != 0:
        fail(f"arm4: wrong ledger counts {c4}")
    passed("arm4 unnoted removal union-retained (never-drop holds), classed retained=1")

    # Arm 4b — fail-closed drops channel: a malformed drop note refuses the
    # refresh outright and appends no ledger line.
    bad_drops = f"{tmp}/bad-drops.txt"
    open(bad_drops, "w").write("2026/10/14 prov/beta malformed note\n")
    e = f"{tmp}/e.json"
    led5 = f"{tmp}/led5.jsonl"
    shutil.copyfile(base, e)
    before_e = open(e, "rb").read()
    run(corr, e, day, bad_drops, led5, False)
    if open(e, "rb").read() != before_e:
        fail("arm4b: refused refresh modified the bundle")
    if os.path.exists(led5):
        fail("arm4b: refused refresh appended a ledger line")
    passed("arm4b malformed drop note refuses, bundle untouched, no ledger line")

    # Arm 5 — honored removal: a dated manual-drop note lets the key leave
    # and records a removal event (rm-588 + rm-1440 composed).
    d = f"{tmp}/d.json"
    led4 = f"{tmp}/led4.jsonl"
    shutil.copyfile(base, d)
    run(drop, d, day, f"{fixtures}/drops-honored.txt", led4, True)
    got = json.load(open(d))
    if "prov/beta" in got:
        fail("arm5: honored drop did not remove the key")
    c5 = last_line(led4)["counts"]
    if c5["removal"] != 1:
        fail(f"arm5: wrong ledger counts {c5}")
    passed("arm5 honored manual removal recorded as a removal event")

    # Arm 6 — ledger schema: every produced line parses and carries the
    # change-class contract fields.
    for path in (led1, led2, led3, led4):
        for line in open(path).read().splitlines():
            rec = json.loads(line)
            for field in ("date", "source_md5", "counts", "events"):
                if field not in rec:
                    fail(f"arm6: ledger line missing {field}")
    passed("arm6 ledger lines parse with date/source_md5/counts/events")

    # Arm 7 — event-cap ordering (review fix F6): a refresh above
    # LEDGER_EVENT_CAP keeps the FIRST 400 events, so the safety-critical
    # classes must be assembled FIRST — under the old growth-first order a
    # correction flood truncated the field_removal/removal/oscillation
    # records away. counts must stay complete under the cap and the loss
    # must be disclosed via truncated_events.
    src_base = json.load(open(base))
    big_b = {"_snapshot": {"date": "2026-10-01", "models": 0, "source": "fixture"}}
    big_l = {"_snapshot": {"date": "2026-10-01", "models": 0, "source": "fixture"}}
    big_b.update(src_base)
    big_l.update(json.loads(json.dumps(src_base)))
    del big_l["prov/beta"]["litellm_provider"]  # exactly one field_removal event
    for i in range(500):
        key = f"prov/flood-{i:03d}"
        big_b[key] = {
            "input_cost_per_token": 1e-06,
            "output_cost_per_token": 2e-06,
            "cache_creation_input_token_cost": 0,
            "cache_read_input_token_cost": 0,
            "mode": "chat",
            "litellm_provider": "prov",
        }
        big_l[key] = dict(big_b[key], input_cost_per_token=2e-06)  # correction flood
    json.dump(big_b, open(f"{tmp}/big-base.json", "w"))
    json.dump(big_l, open(f"{tmp}/big-live.json", "w"))
    f7 = f"{tmp}/f7.json"
    led7 = f"{tmp}/led7.jsonl"
    shutil.copyfile(f"{tmp}/big-base.json", f7)
    run(f"{tmp}/big-live.json", f7, day, empty_drops, led7, True)
    last = last_line(led7)
    if last["counts"]["correction"] != 500 or last["counts"]["field_removal"] != 1:
        fail(f"arm7: counts not complete under the cap: {last['counts']}")
    if len(last["events"]) != 400 or last.get("truncated_events") != 101:
        fail(
            f"arm7: cap not applied: {len(last['events'])} events kept, "
            f"truncated={last.get('truncated_events')}"
        )
    classes = [e["class"] for e in last["events"]]
    if "field_removal" not in classes:
        fail("arm7: safety-critical field_removal event was truncated away (cap ordering broken)")
    if classes[0] != "field_removal":
        fail(f"arm7: safety-critical classes must lead the event list, first is {classes[0]!r}")
    passed("arm7 event cap: safety-critical class survives, counts complete, truncation disclosed")


mode = sys.argv[1]
if mode == "refresh":
    refresh(sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5], sys.argv[6])
elif mode == "selftest":
    selftest(sys.argv[2], sys.argv[3])
else:
    sys.exit(f"REFUSED: unknown core mode {mode!r}")
PYCORE
)"

# rm-901 review fix F4: exactly zero or one argument is accepted — a
# second argument after a recognized flag is REFUSED (rc 2), not silently
# ignored. Pre-fix, `--check extra` ran the check while silently dropping
# `extra`; the header contract ("any other argument is REFUSED") now holds
# for every argv shape, and a stray second argument can never trigger a
# network fetch under a guard flag.
if [[ "$#" -gt 1 ]]; then
    usage >&2
    echo "REFUSED: exactly one argument allowed, got $#: $* — see --help" >&2
    exit 2
fi

case "${1:-}" in
    "")
        ;; # no arguments: the documented refresh path
    --help | -h)
        usage
        exit 0
        ;;
    --check)
        # rm-901: report-only drift check — zero writes, zero side effects.
        tmp="$(mktemp)"
        trap 'rm -f "$tmp"' EXIT
        if [[ -n "$src" ]]; then
            if [[ ! -r "$src" ]]; then
                # rm-1440 review fix F7: an unreadable capture is INFRA (rc 2,
                # the drift-check.sh exit contract) — never a set -e cp death
                # at rc 1 that collides with "actionable drift" semantics.
                echo "update-snapshot.sh: infra error: LITELLM_SNAPSHOT_SRC not readable: $src" >&2
                exit 2
            fi
            cp -- "$src" "$tmp"
            echo "check source: captured file $src (offline check)"
        else
            curl -fsSL "$url" -o "$tmp"
            echo "check source: $url (live fetch)"
        fi
        echo "check mode (rm-901): report-only — nothing is written"
        set +e
        bash "$here/drift-check.sh" --live "$tmp"
        check_rc=$?
        set -e
        rm -f "$tmp"
        exit "$check_rc"
        ;;
    --histogram)
        # rm-1440: replay the last ledger line, read-only.
        if [[ ! -s "$ledger" ]]; then
            echo "no refresh ledger yet ($ledger absent or empty) — no refresh has recorded change classes"
            exit 0
        fi
        python3 - "$ledger" <<'PYH'
import json
import sys

lines = open(sys.argv[1]).read().splitlines()
rec = json.loads(lines[-1])
counts = rec.get("counts", {})
parts = " ".join(f"{k}={v}" for k, v in counts.items())
print(f"last refresh (date {rec.get('date', '?')}): {parts}")
print(f"ledger lines: {len(lines)} | events in last line: {len(rec.get('events', []))}")
PYH
        exit 0
        ;;
    --selftest)
        tmpdir="$(mktemp -d -t snapshot-selftest.XXXXXX)"
        trap 'rm -rf "$tmpdir"' EXIT
        echo "update-snapshot --selftest (offline arms, no network, no tracked writes)"
        # rm-901 guard smoke: every non-refresh flag must leave the real
        # bundle byte-identical (the no-side-effect guarantee, pinned).
        pre="$(md5sum "$out" | cut -d' ' -f1)"
        if bash "$here/update-snapshot.sh" --help >/dev/null 2>&1; then
            echo "guard: --help exits rc 0"
        else
            echo "guard: FAIL --help must exit rc 0"
            exit 1
        fi
        if bash "$here/update-snapshot.sh" --typo-flag >/dev/null 2>&1; then
            echo "guard: FAIL a stray argument must be REFUSED"
            exit 1
        else
            echo "guard: stray argument refused (rc non-zero)"
        fi
        if bash "$here/update-snapshot.sh" --histogram >/dev/null 2>&1; then
            echo "guard: --histogram exits rc 0"
        else
            echo "guard: FAIL --histogram must exit rc 0"
            exit 1
        fi
        if bash "$here/update-snapshot.sh" --check extra-stray-arg >/dev/null 2>&1; then
            echo "guard: FAIL a second argument after a recognized flag must be REFUSED"
            exit 1
        else
            echo "guard: second argument after a recognized flag refused (rc non-zero, no fetch)"
        fi
        if LITELLM_SNAPSHOT_SRC="$tmpdir/does-not-exist.json" bash "$here/update-snapshot.sh" --check >/dev/null 2>&1; then
            echo "guard: FAIL an unreadable LITELLM_SNAPSHOT_SRC under --check must exit non-zero"
            exit 1
        else
            rc_capture=$?
            if [[ "$rc_capture" != "2" ]]; then
                echo "guard: FAIL unreadable LITELLM_SNAPSHOT_SRC must be infra rc 2, got $rc_capture"
                exit 1
            fi
            echo "guard: unreadable LITELLM_SNAPSHOT_SRC under --check is infra (rc 2, no fetch, no writes)"
        fi
        post="$(md5sum "$out" | cut -d' ' -f1)"
        if [[ "$pre" != "$post" ]]; then
            echo "guard: FAIL the bundle changed under a non-refresh flag"
            exit 1
        fi
        echo "guard: bundle byte-identical under all non-refresh flags"
        python3 - selftest "$fixtures" "$tmpdir" <<<"$core_py"
        echo "selftest: ALL ARMS PASS"
        exit 0
        ;;
    *)
        usage >&2
        echo "REFUSED: unknown argument '$1' — see --help for the accepted flags" >&2
        exit 2
        ;;
esac

tmp="$(mktemp)"
trap 'rm -f "$tmp" "$out.new"' EXIT

if [[ -n "$src" ]]; then
    if [[ ! -r "$src" ]]; then
        # review fix F7 (refresh twin of the --check guard): infra rc 2, not a
        # silent cp failure under set -e.
        echo "update-snapshot.sh: infra error: LITELLM_SNAPSHOT_SRC not readable: $src" >&2
        exit 2
    fi
    cp -- "$src" "$tmp"
    echo "refresh source: captured file $src (offline refresh)"
else
    curl -fsSL "$url" -o "$tmp"
    echo "refresh source: $url (live fetch)"
fi

python3 - refresh "$tmp" "$out" "$date" "$drops" "$ledger" <<<"$core_py"
