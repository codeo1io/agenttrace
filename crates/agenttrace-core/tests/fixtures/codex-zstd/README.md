# Codex zstd rollout fixture (rm-1005)

`rollout-with-usage.jsonl.zst` is the committed
`../codex-legacy/rollout-with-usage.jsonl` corpus (rm-716, tokscale
#1405 envelope) run through real `zstd -19`:

```
zstd -19 ../codex-legacy/rollout-with-usage.jsonl
```

sha256 (rm-542 convention):

```
5cb6306d816beaee7d57a790c503f51a36e96c6a04fa084d988a4e5015d36159  rollout-with-usage.jsonl.zst
```

Why: Codex ≥0.152 persists rollouts as zstd frames (magic
`28 B5 2F FD`, upstream PR #41357). `parse_file` decodes them
in-process via ruzstd (pure Rust — no C toolchain, no host zstd
dependency) bounded by a 256 MiB inflate cap; `tests/
zstd_decode_contract.rs` pins the twin-parity contract (decoded
accounting == plain-JSONL accounting) plus corrupt/truncated
fail-closed behavior, and assembles a RAW-block frame byte-by-byte in
Rust so the suite never depends on a host zstd binary. The default
discovery walk still excludes `.zst` (the file-name filter gates on
`.jsonl`/`.json`) — this arm serves explicit file paths and pipes,
matching the upstream default-off datum.
