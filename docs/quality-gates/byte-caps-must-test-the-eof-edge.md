# Byte/line caps must test the EOF edge, not just the mid-stream refusal

**Class:** quality-gate (test-shape) · **Minted:** 2026-10-11, run e4eb22544532 cycle 1 compound (attempt c8d175d0) · **Evidence:** rm-921 residual (assess 285d2fce live PoC → implement 3b613f9a rider)

## The failure shape

`read_message_line` enforced a 1 MiB transport cap correctly for the mid-stream case:
an over-cap line followed by more input produced the disclosed JSON-RPC refusal. But
the EOF return block consulted `line.is_empty()` **before** the `overlong` flag. When
the cap-crossing accumulate was the stream's final act, the code cleared the buffer
and set `overlong = true` — and at EOF the empty-line branch matched first, so the
host received **silence and exit 0** for a message it could not know was dropped.

Live proof (assess 285d2fce): exactly 1,048,577 bytes with no trailing newline → empty
output, clean exit; the same payload +8,192 bytes → the disclosed refusal. The
behavior changed with payload SIZE alone, not content.

## The rule

Every cap/limit guard needs a boundary test arm at the **terminator edge**, not only
the mid-stream arm:

1. exact `cap + 1` bytes **with** the terminator present (mid-stream refusal), and
2. exact `cap + 1` bytes **without** any terminator, EOF immediately after (the
   truncation edge), asserting the refusal still fires.

The second arm is the one that catches flag-vs-emptiness ordering bugs in the EOF
branch — the exact defect class here. A cap test without it proves the common path
and silently trusts the edge.

## Why this generalizes in this repository

Three cap families landed this month: the MCP line cap (rm-923), the statusline
journal byte cap (rm-922), and now the EOF-edge fix (rm-921 rider). Any future cap —
discovery file-size caps (rm-927 family), network-body caps (rm-052 family) — must
carry both arms in its red-first pair. The fixture is cheap: a sized payload built by
truncation (`sized.truncate(cap + 1)`), no terminator, pipe closed.

Reference implementation: `crates/agenttrace-cli/tests/mcp_server.rs::overlong_line_terminated_by_eof_is_refused_not_silently_dropped`.
