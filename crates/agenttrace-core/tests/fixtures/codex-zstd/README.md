# codex-zstd — compressed rollout fixtures (rm-992, rm-1516)

Codex stores rollout transcripts as zstd frames: the files keep `.jsonl`
names but start with the zstd magic `28 B5 2F FD` (reader tolerance
since upstream ≥0.152, PR #41357 merged 2026-10-13). Stable Codex
0.162.x migrates cold rollouts under `~/.codex/sessions`/
`~/.codex/archived_sessions` to compressed storage — `.jsonl.zst`
names — while writing compressed rollouts by default remains
feature-flagged upstream (#53732 rode the 0.163 alphas and the flag
was still off at tag rust-v0.163.0; compressed user data on stable is
real regardless — ccusage #1888, fixed #1890).

- `rollout.jsonl` — single-frame compression of the golden plain twin
  `../codex-custom-tools/rollout.jsonl`. Pins decode parity: the
  compressed parse must equal the twin's parse.
- `rollout.jsonl.zst` (rm-992, run d57c2e56 cycle 1) — the SAME frame as
  `rollout.jsonl` under the migrated `.jsonl.zst` name (a byte-identical
  copy), so admission-by-name and decode-by-content are provable
  independently of the frame itself.
- `rollout-bomb.jsonl` — single frame claiming more than the 64 MiB
  decode cap (40 MiB of incompressible noise). Pins the per-frame
  over-cap refusal.
- `rollout-multiframe.jsonl` (rm-1516) — the SAME golden twin split at a
  line boundary (25 + 26 lines) and re-compressed as TWO concatenated
  zstd frames — the shape Codex's append-via-materialize produces.
  `zstd -dc` on it is byte-identical to the twin (verified at
  generation; `zstd -l` reports 2 frames, 0 skips). Pins that the
  decode reaches true EOF: before rm-1516 the decoder stopped after
  frame 1 and the parser billed a half journal (19 of 38 events).
- `rollout-bomb-multiframe.jsonl` (rm-1516) — two frames of 40 MiB of
  zeros each: each alone under the 64 MiB cap, 80 MiB together. Pins
  that the cap bounds the TOTAL decompressed output across frames.

Regenerate (zstd CLI ≥1.5; parity oracle included):

```sh
FIX=crates/agenttrace-core/tests/fixtures
TWIN=$FIX/codex-custom-tools/rollout.jsonl
head -n 25 "$TWIN" > half-a.jsonl
tail -n 26 "$TWIN" > half-b.jsonl
zstd -f -q -19 half-a.jsonl half-b.jsonl
cat half-a.jsonl.zst half-b.jsonl.zst > "$FIX/codex-zstd/rollout-multiframe.jsonl"
zstd -dc "$FIX/codex-zstd/rollout-multiframe.jsonl" | cmp - "$TWIN"   # must be byte-identical
head -c $((40*1024*1024)) /dev/zero > zeros40.bin
zstd -f -q -19 zeros40.bin
cat zeros40.bin.zst zeros40.bin.zst > "$FIX/codex-zstd/rollout-bomb-multiframe.jsonl"
cp "$FIX/codex-zstd/rollout.jsonl" "$FIX/codex-zstd/rollout.jsonl.zst"  # the migrated-name twin
```

Skippable-frame and trailing-garbage arms are synthesized in the tests
(`zstd_rollout_decode.rs`) from `rollout.jsonl` bytes directly, so no
binary blob is needed for them.
