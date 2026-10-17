# Refusal numerals must derive from the cap constant

**When**: an error or refusal message restates a numeric limit that also
exists as a constant (byte caps, entry caps, retry counts, timeouts).

**Why**: the zstd rollout decode-cap refusal hardcoded "64 MiB" as a string
literal (parser.rs, over-cap bail) while the cap itself was
`ZSTD_ROLLOUT_DECODE_MAX_BYTES`. A later cap change would have shipped a
lying error message — the exact failure mode the `MCP_INPUT_MAX_BYTES` arm
already fixed once (mcp.rs:104-112). Duplicated numerals rot silently because
no type system checks strings.

**Fix**: compute the displayed numeral from the constant at the point of
formatting (e.g. `ZSTD_ROLLOUT_DECODE_MAX_BYTES / (1024 * 1024)`), and pin the
coupling with a const-tracking test that formats the refusal and asserts the
constant's value appears, so editing the constant without updating the
message fails a test instead of shipping.

**Evidence**: rm-1025 (implemented 2026-10-16, run e775cac5 cycle 1; test
`over_cap_refusal_numeral_tracks_the_constant`, parser.rs
`zstd_cap_label_tests`; precedent mcp.rs:104-112 landed rm-041 family).
Audit pattern: grep message-format sites for numeric literals matching any
`*_MAX_*` constant's value.
