# A decoder crate's happy path is not the format contract — the writer's own reader is

**Class:** reliability prevention · **First fleet instance recorded:** run 1e4f6b95 cycle 4 (2026-10-16) · **Prevention owner:** every implement-phase delegate touching a foreign-format decode lane

## Failure shape

`agenttrace` decoded Codex compressed rollouts with `ruzstd` and, on every
multi-frame (concatenated-zstd) rollout, silently parsed **frame 1 only** —
billing a half journal with **zero disclosure**. No test caught it because no
fixture was multi-frame; the fixture pair shipped with the original lane
(`rm-992`) was single-frame by construction. Worse: the decode-failure error
message recommended `zstd -d` — a command that decodes *all* frames — so our
own guidance produced streams our parser under-parsed.

## Root cause mechanics

- `ruzstd` 0.9.1's `StreamingDecoder` is **single-frame by crate contract**
  (`streaming_decoder.rs:22-25`: *"To decode all the frames in a finite
  stream, the calling code needs to recreate the instance of the decoder"*;
  its `Read` impl returns `Ok(0)` once frame 1 finishes — indistinguishable
  from EOF at the call site that doesn't know the contract).
- The **writer's own reader** is the real format authority: Codex's
  `seekable_reader.rs:139-141` explicitly tolerates concatenated zstd frames
  ("a known first-frame size is a lower bound even for concatenated zstd
  frames"), and every Codex read path rides libzstd's multi-frame-default
  `Decoder`. Codex *produces* concatenated frames because appends happen by
  materializing the journal and re-encoding (`compression.rs`
  materialize-for-append; `recorder.rs:1764-1775`).
- Two decoders therefore implement different contracts for "a zst file";
  adopting the crate's happy path silently chose the narrower one.

## Prevention rules

1. **Before adopting any decoder for a foreign-tool format, read the
   writer's own reader** and pin the lane to *its* contract, not the
   crate docs. Ask explicitly: does the producer ever emit multiple
   frames / records / streams per file?
2. **Every new decode lane ships a fixture exercising the format's edge
   shape** (here: a two-frame rollout, `rollout-multiframe.jsonl`) —
   single-shape fixtures prove nothing about shape handling.
3. **Red-first the edge**: the fix landed against a failing test
   (19 of 38 events parsed pre-fix, zero disclosure).
4. **Parity-oracle against the reference implementation**: the golden
   property is `agenttrace output == the same output after
   \`zstd -dc\`` on the twin — and error semantics match too (trailing
   garbage refused with an offset, skippable frames skipped), because
   users will compare against `zstd -d` behavior when our message names it.
5. Budget caps must bound the **total across frames**, not per frame —
   a per-frame cap lets a multi-frame file smuggle past it a frame at a
   time (the 64 MiB cap now spans the whole decode).

## Fleet instance (evidence)

- Mint: ROADMAP `rm-1516` (run 1e4f6b95 cycle 4, base 16dcc9a); fix in
  `decode_zstd_rollout_bounded` (per-frame decoder recreation to true EOF);
  tests in `crates/agenttrace-core/tests/zstd_rollout_decode.rs`
  (concatenated / total-cap / trailing-offset / skippable-frame arms).
- Upstream proof: codex-rs `seekable_reader.rs:61,:91,:139-141,:169` and
  `compression.rs:105-149` (research dossier 7f41f39d RC1).
