# reemission-thinking fixtures (rm-891, run ac3ac300 cycle 2)

Minimal committed forms of the assess-phase PoC corpora (run c92f079f F2:
`poc-reasoning/` vs `poc-reasoning-ctrl/`, and its fold-dedup probe). Every
fixture is one claude-code JSONL conversation with a keyed assistant message
(`msg_01`/`msg_02`) re-emitted on a later line.

- `non-monotonic.jsonl` — the finding itself: the FINAL re-emission of
  `msg_01` carries only the text block and omits the thinking block the
  earlier snapshot had. Pre-fix the fold's replace-in-place set
  `reasoning = ""`, so `reasoning_blocks` collapsed to 0. sha256
  `efffbc77ee80114d5395f8c470aba82a8f7de3ad07ef4343b0ed904b4e193dac`.
- `control-keeps-thinking.jsonl` — same conversation, but the final
  re-emission KEEPS the thinking block (monotonic snapshots, richer final).
  One turn, one reasoning block, before and after the fix. sha256
  `5cbbd561b1d7f85079b5fdc03c0f94c4daf3686d7d86eb7e6c10854bc94c883e`.
- `monotonic-growth.jsonl` — growth direction: the first emission has only
  text, the final adds thinking. The latest snapshot's non-empty fields must
  win (union must not over-retain the thin first snapshot's content). sha256
  `9a6cee4dacd84e872f94d262d8800b4b835a35c4bc1a3aad807b4057bb0f0189`.
- `empty-tool-ids.jsonl` — one keyed message (`msg_02`) whose emission
  carries TWO distinct `tool_result` blocks, both with an EMPTY
  `tool_use_id`, re-emitted identically. Pre-fix the dedup key
  `(message_id, tool_use_id)` collapsed both distinct results into the
  first (`tool_results` = 1); ordinal keying keeps both distinct while the
  re-emission still folds away (`tool_results` = 2). sha256
  `ab2d6f10b49083fa419c966a856ae22174963e05a1f3835ec88ecb6966e68f48`.

sha256 pins per the rm-542 fixture convention.
