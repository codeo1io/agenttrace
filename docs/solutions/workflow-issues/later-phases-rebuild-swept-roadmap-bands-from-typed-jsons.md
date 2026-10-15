# Later phases rebuild swept roadmap bands from typed JSONs

**Class:** workflow prevention · **First fleet instance recorded:** run 13212e2f cycle 4 (2026-10-12) · **Prevention owner:** every roadmap/compound-phase delegate

## Failure shape

A roadmap phase builds its band (banner + evidence riders) in its delegate scratch and mirrors it under `/tmp`; by the time the compound phase of the same cycle arrives, BOTH copies — and sometimes `.cache/at-*` mirrors too — have been janitor-swept. The cycle's roadmap delta then exists nowhere durable except the roadmap phase's typed result JSON. Run aa36ecb0 cycle 3 (2026-10-15) hit the full triple: `delegate/2d55b308…-scratch/` gone, `/tmp/at-roadmap-2d55b308/` gone, and of the implement/targeted/full `.cache` mirrors two of three were also gone — while every typed `delegate/<attempt>.json` survived.

## Root cause mechanics

- Delegate scratch dirs and `/tmp` paths are actively reaped mid-run (box janitor; fleet constraint notes). `/home/agent/.cache/at-*` is *more* durable but not guaranteed — whole subdirs vanish.
- The typed result JSONs under the spool root are the artifact class the engine itself preserves; they are the only carrier with fleet-wide survival.
- Roadmap phases that record their rider texts only inside scratch (patch/postimage files) become unrecoverable byte-exact; phases that embed the full rider text in the typed result's summary/artifact record stay recoverable in content.

## Prevention rules

1. **Embed, don't only link.** A roadmap (or band-producing) phase must carry the full rider/banner text inside its typed PhaseResult — the summary or an artifacts entry — not just the scratch paths. Run aa36ecb0's roadmap phase did this and was the reason its band could be rebuilt.
2. **Later phases rebuild from typed records and declare it.** When the compound phase finds the band swept, it re-emits the riders from the roadmap typed JSON, marks the re-mint provenance in the new banner ("content-faithful, not byte-identical to the swept postimage md5 …"), and never silently pretends continuity.
3. **Mirror anyway, cheaply.** Keep the `.cache/at-<attempt>/` mirror (it survives often, and never harms), but treat it as best-effort — rule 1 is the guarantee.
4. **Forensics before rebuild.** Confirm the sweep (scratch absent, /tmp absent, typed JSON present) and record the three checks in the compound result, so a future reader can distinguish "swept" from "never existed".

## Evidence

- 13212e2f cycle 4 (2026-10-12): mints existed nowhere durable; re-emitted from older typed result JSONs.
- aa36ecb0 cycle 3 (2026-10-15): roadmap band (+7/−0, postimage ac66734f) swept; re-minted from `delegate/2d55b308….json` to a new +7/−0 band (postimage a7903bbb) with re-mint provenance declared in the banner; chain wall(1d59adda) ← band ← compound verified by `git apply` round-trip.
