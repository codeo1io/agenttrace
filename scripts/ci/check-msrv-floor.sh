#!/usr/bin/env bash
# MSRV floor gate (rm-386, cycle 5).
#
# The workspace advertises `rust-version` in Cargo.toml. That number is a
# contract with every downstream builder; it is truthful only while it stays
# >= the greatest rust_version declared by any locked dependency AND >= what
# the workspace's own code needs. The `msrv` job in .github/workflows/ci.yml
# proves the advertised floor actually BUILDS; this gate catches the drift a
# build cannot:
#
#   1. manifest floor < dep-greatest floor (a dependency bump silently raised
#      the real floor above the advertised one)              -> FAIL
#   2. the msrv CI lane's toolchain selector no longer equals
#      the manifest rust-version (advertised floor without its build) -> FAIL
#   3. Cargo.lock md5 no longer matches the recorded table    -> WARN
#      (table is stale; regenerate, see below)
#
# Floor table: scripts/ci/msrv-floor.txt records the dep-greatest floor and
# the lock md5 it was computed from. Regenerating is a network operation
# (crates.io /api/v1/crates/{name}/{version} -> rust_version for every
# [[package]] in Cargo.lock; unset = unconstrained), so it is done per
# maintenance cycle rather than in CI:
#
#   python3 - <<'PY'
#   import json,re,time,urllib.request,hashlib
#   lock=open('Cargo.lock').read()
#   print('lock md5:',hashlib.md5(lock.encode()).hexdigest())
#   pkgs=re.findall(r'\[\[package\]\]\nname = "([^"]+)"\nversion = "([^"]+)"',lock)
#   best=(0,0,0);holders=[]
#   for i,(n,v) in enumerate(p for p in pkgs if not p[0].startswith('agenttrace')):
#       try:
#           d=json.load(urllib.request.urlopen(urllib.request.Request(
#               f'https://crates.io/api/v1/crates/{n}/{v}',
#               headers={'User-Agent':'agenttrace-msrv-floor/1.0'}),timeout=15))
#           rv=d.get('version',{}).get('rust_version')
#           if rv:
#               mm=tuple(int(x) for x in re.findall(r'\d+',rv)[:3])
#               if mm>best: best,holders=mm,[f'{n} {v}']
#               elif mm==best: holders.append(f'{n} {v}')
#       except Exception: pass
#       if i%10==9: time.sleep(0.15)
#   print('dep_greatest_floor:','.'.join(map(str,best)))
#   print('floor_holders:','; '.join(holders))
#   PY
#
# …then update msrv-floor.txt (lock md5 + floor + holders) by hand.

set -euo pipefail
cd "$(dirname "$0")/../.."

table=scripts/ci/msrv-floor.txt
[ -f "$table" ] || { echo "check-msrv-floor: missing $table" >&2; exit 1; }

manifest="$(sed -n 's/^rust-version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
[ -n "$manifest" ] || { echo "check-msrv-floor: no rust-version in Cargo.toml" >&2; exit 1; }

floor="$(sed -n 's/^dep_greatest_floor: //p' "$table")"
[ -n "$floor" ] || { echo "check-msrv-floor: no dep_greatest_floor in $table" >&2; exit 1; }

vercmp() {  # vercmp A B -> prints lt|eq|gt (numeric, per component)
  awk -v A="$1" -v B="$2" 'BEGIN {
    n = split(A, a, "."); m = split(B, b, ".")
    for (i = 1; i <= (n > m ? n : m); i++) {
      x = (i <= n ? a[i] + 0 : 0); y = (i <= m ? b[i] + 0 : 0)
      if (x < y) { print "lt"; exit }
      if (x > y) { print "gt"; exit }
    }
    print "eq"
  }'
}

case "$(vercmp "$manifest" "$floor")" in
  lt) echo "check-msrv-floor: FAIL — Cargo.toml rust-version $manifest is BELOW the dep-greatest floor $floor" \
        "(see $table; bump rust-version and the msrv CI lane together)" >&2
      exit 1 ;;
  eq) echo "check-msrv-floor: OK — rust-version $manifest == dep-greatest floor $floor" ;;
  gt) echo "check-msrv-floor: OK — rust-version $manifest exceeds dep floor $floor (workspace code requirement; see $table)" ;;
esac

msrv_job=".github/workflows/ci.yml"
if grep -q "cargo +${manifest%.0} check --locked\|cargo +$manifest check --locked" "$msrv_job"; then
  echo "check-msrv-floor: OK — msrv CI lane builds at +$manifest"
else
  echo "check-msrv-floor: FAIL — msrv CI lane in $msrv_job does not run \`cargo +$manifest check --locked\`;" \
        "the advertised floor has no build proof" >&2
  exit 1
fi

lock_md5="$(md5sum Cargo.lock | cut -d' ' -f1)"
table_md5="$(sed -n 's/^cargo_lock_md5: //p' "$table")"
if [ "$lock_md5" != "$table_md5" ]; then
  echo "check-msrv-floor: WARN — Cargo.lock ($lock_md5) differs from the table's basis ($table_md5);" \
        "regenerate $table at the next maintenance touch"
fi
exit 0
