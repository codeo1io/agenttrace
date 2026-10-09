# Integrity pins must be command-emitted, never hand-typed

**Class:** numeric-provenance failure (evidence fabrication without artifact tampering).
**Found:** 2026-10-09/10, repository-maintenance `3f178c838e6f4d9491601c3e95616657` cycle 1, run `57c49133881840c4b7f81dba107d6333` (roadmap phase `ed44938c6a9e46fe865d1d801ecf75e6`; caught by stewardship phase `091d54269adb47509f68b7ce31d63bd6`; compounded 2026-10-10 by attempt `58091d4d80b64ea39a6fb93f344f9ac4`).

## Symptom

A phase record claimed integrity pins (sha256/md5) for artifacts it had produced or verified. None of the three pins matched the artifacts — not truncated variants, not stale values: the hex strings were confabulated wholesale. The underlying work (patch, postimage, roundtrip proof) was fully valid; only the recorded numbers were wrong. Anyone later re-verifying "against the record" would have concluded the artifacts were tampered with.

## Incident specifics

- Claimed: patch sha256 `449b9435…`, postimage md5 `30790142…`, pristine md5 `7460221a…` — none matched.
- Authoritative (re-emitted by command at compounding): pristine sha256 `6bd1165511632445951993e35bcbae9c1b184f5795b3b50f6efa671bb78c394b` (md5 `36810af78afc04c766170f858610745b`), roadmap postimage sha256 `64f0b12db4c1a173cc43c1b4c116101e288caf97ab1e6f36b1e4a6aebc6b4c98` (md5 `917582b93cd0dc05b3e9f9120e3b58d9`). Also off by one: the recorded pristine line count (3876 vs actual 3875).
- The consumed envelope could not be rewritten; the correction traveled as a dated correction in ROADMAP-NOTES.md naming both the void and authoritative values, and later gates were told to prefer it.

## Detection recipe (all three legs, plus the tamper-vs-typo split)

1. Any later phase touching a pinned artifact **re-runs the hash first** and reconciles against the record before building on it.
2. On mismatch, prove tampering-vs-typo with: artifact mtimes vs phase timestamps (unchanged mtime = never rewritten), HEAD/porcelain stability, and a full roundtrip re-proof of the artifact itself.
3. If the chain re-proves clean, the pins were confabulated: amend the durable notes with a dated correction naming the void values AND the authoritative ones. The next phase's record carries the correction forward.

## Prevention rule

- **Every pin is emitted by a command in the SAME tool call that writes or verifies the record** — e.g. `sha256sum artifact >> notes.md` inline, or pasted from output captured seconds earlier with the file still on disk. Never compose a hash from memory, even one you "just saw".
- A record file **cannot embed its own hash**: the hash block's bytes change the file after any pre-computed pin. Embedded blocks pin only the OTHER artifacts; the record's own final hash is pinned externally (scratch + phase result), computed by command AFTER the file is final. (Variant hit live by run `075d70c67abf` compound `e775e6fa` the same week.)
- Distrust transcribed numbers generally: line counts and "verbatim" strings copied by hand fail the same way (see the off-by-one line count above, and the verbatim-contract doc for the string case).

## Related classes (do not conflate)

- `conductor-validation-evidence-is-a-verbatim-contract.md` — strings that must be copied, not retyped (dispatch digests/commands).
- Phantom-amendment failures (claiming an edit that was never written): verify by token-grep of the claimed text, artifact mtime vs phase timestamp, and scratch sweep — a "verification" that greps for a phrase that never existed proves nothing.
- Adopted-attempt prose transposition (#17495-class): adopting a prior attempt's claims without re-deriving its numbers.
