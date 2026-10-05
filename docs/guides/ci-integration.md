# CI Integration

Use `agenttrace --overview` as a quality gate for AI agent sessions in pull requests or nightly jobs.

The goal is to catch agent workflow regressions before they become invisible cost:

- a PR's agent run starts hanging or retrying tools
- a nightly automation job burns more tokens than usual
- a team switches agent tools and loses session health visibility
- a local-first project wants CI evidence without uploading prompts or raw logs to a hosted trace service

Start with report-only artifacts, then turn on blocking thresholds once the team knows its normal health and tool failure range.

Use `--range today|7d|30d|all`, `--project`, `--source`, and
`--model-filter` to keep reports and gates on the same explicit scope. JSON
reports include `data_health` and `by_project` so automation can distinguish a
healthy run from missing, skipped, or fallback-priced data.

Long-term history is opt-in. `--preserve-history` stores only derived metrics
(time, project label, source, model, tokens, cost, health, and anomaly labels)
under the user data directory. It does not copy session paths, prompts, replies,
or tool arguments. Add `--include-history` when a report should merge those
preserved metrics with live sessions.

## Local Check

```bash
agenttrace --overview \
  --fail-under-health 80 \
  --fail-on-critical \
  --max-tool-fail-rate 15
```

The command exits with code `2` when a gate fails. Add `-f json -o agenttrace-overview.json` when CI should upload machine-readable data, `-f markdown -o agenttrace-overview.md` when the report should be pasted into a PR comment, or `-f html -o agenttrace-overview.html` for a self-contained visual artifact.

With `-o`, agenttrace keeps the report body on stdout while also writing the file.
Saved-file confirmations and gate diagnostics are written to stderr; this keeps JSON stdout
machine-readable and lets Markdown or HTML output remain useful for logs and previews.

For the first few runs, keep the job non-blocking while still collecting evidence:

```bash
agenttrace --overview -f markdown -o agenttrace-overview.md || true
agenttrace --overview -f html -o agenttrace-overview.html || true
```

When the output matches what the team cares about, enable blocking checks:

```bash
agenttrace --overview -f json \
  --fail-under-health 80 \
  --fail-on-critical \
  --max-tool-fail-rate 15 \
  -o agenttrace-overview.json
```

To compare a current run with a local CI baseline artifact, keep a previous
`--overview -f json` report and pass it back with explicit delta thresholds:

```bash
agenttrace --overview -f json \
  --baseline agenttrace-baseline.json \
  --baseline-max-duration-delta-pct 10 \
  --baseline-max-cost-delta-pct 15 \
  --baseline-max-token-delta-pct 20 \
  -o agenttrace-overview.json
```

The JSON report includes `baseline_comparison` with deterministic fields for
duration, cost, token deltas, new failure families, broader tool/file surfaces,
new tool authority categories, and new high-authority tool use. Baseline reports
must be local JSON artifacts from the same agenttrace version.

Each `recent_sessions` item can also include local-only project metadata such as
`cwd` when the source log exposes it, plus a conservative
`possible_cost_driver` note when existing evidence points to context pressure,
large parameters/output, retry loops, tool failures, or high tokens per turn.
These notes are diagnostic clues, not guaranteed savings claims.

For lightweight local lookup without adding an indexer, `agenttrace --search`
matches session metadata, source/model names, cwd/path metadata, tools, files,
anomaly labels, authority categories, and diagnostic evidence. It does not
search prompt or assistant message text by default:

```bash
agenttrace --search billing
agenttrace --search internal/ws -f json
```

## GitHub Actions

```yaml
name: Agenttrace

on:
  pull_request:
  workflow_dispatch:

jobs:
  agenttrace:
    runs-on: ubuntu-latest
    steps:
      # actions/checkout v7.0.0 — pinned like the example workflow this
      # guide mirrors (mutable tags can be moved after the fact).
      - uses: actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0
        with:
          fetch-depth: 0
      - name: Install agenttrace
        run: |
          # rm-417 residual: this guide teaches the same supply-chain rule
          # examples/github-actions/agenttrace-health-gate.yml enforces (gated
          # by scripts/ci/check-example-workflows.sh): never curl|sh from a
          # moving ref — pin a release tag and verify the published checksum.
          # Checksums below are the ones published in the v0.9.0 release
          # notes, same as the example workflow.
          set -euo pipefail
          OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
          case "$(uname -m)" in
            x86_64)          ARCH=amd64 ;;
            aarch64 | arm64) ARCH=arm64 ;;
          esac
          ASSET="agenttrace-${OS}-${ARCH}"
          case "${OS}-${ARCH}" in
            linux-amd64)  SUM=ff26324f2114e1757babbbb9d7d0729f80d58073224dd20729c893e6d70a7ad8 ;;
            linux-arm64)  SUM=253bd6c5e2f659477f057ff4ff28728823259eac9cf3e14328d75ba936f6f40f ;;
            darwin-amd64) SUM=af84d9fe5770f2fc93c8125491e87ecea7fb93c7188c668347445341fa6f5a66 ;;
            darwin-arm64) SUM=f29fe44bb5039f5441d79fe53c6893788685914f48910083c88ed7337141c2b3 ;;
          esac
          ASSET_PATH="${RUNNER_TEMP}/${ASSET}"
          curl -fsSL --max-time 120 -o "${ASSET_PATH}" \
            "https://github.com/luoyuctl/agenttrace/releases/download/v0.9.0/${ASSET}"
          echo "${SUM}  ${ASSET_PATH}" | sha256sum -c -
          mkdir -p "${RUNNER_TEMP}/agenttrace-bin"
          install -m 0755 "${ASSET_PATH}" "${RUNNER_TEMP}/agenttrace-bin/agenttrace"
          echo "${RUNNER_TEMP}/agenttrace-bin" >> "$GITHUB_PATH"
      - name: Check agent session health
        run: |
          agenttrace --overview -f json \
            --fail-under-health 80 \
            --fail-on-critical \
            --max-tool-fail-rate 15 \
            -o agenttrace-overview.json
      - name: Compare against local baseline
        if: hashFiles('agenttrace-baseline.json') != ''
        run: |
          agenttrace --overview -f json \
            --baseline agenttrace-baseline.json \
            --baseline-max-duration-delta-pct 10 \
            --baseline-max-cost-delta-pct 15 \
            --baseline-max-token-delta-pct 20 \
            -o agenttrace-overview.json
          # Since the pass-7 fix (P7-3), a breach of any --baseline-max-*
          # threshold fails the step with exit 2, exactly like
          # --fail-under-health — previously the booleans sat unread in
          # the JSON while the step stayed green. Add --no-baseline-gate
          # to keep the comparison informative without failing the build.
      - name: Write Markdown summary
        if: always()
        run: |
          agenttrace --overview -f markdown -o agenttrace-overview.md || true
          agenttrace --overview -f html -o agenttrace-overview.html || true
      # actions/upload-artifact v7.0.0 — pinned like the example workflow.
      - uses: actions/upload-artifact@bbbca2ddaa5d8feaa63e36b76fdaad77386f024f
        if: always()
        with:
          name: agenttrace-overview
          path: |
            agenttrace-overview.json
            agenttrace-overview.md
            agenttrace-overview.html
```

Tune thresholds per repository. A stricter team can start with health `90` and tool failure rate `5`; early adopters may start at `70` and `25` to avoid blocking useful experimentation.

## Repository CI Gates

This repository also runs agenttrace against its own demo and docs surfaces so repetitive Agent validation becomes a stable CI contract.

The project CI builds `target/release/agenttrace` and runs:

```bash
scripts/ci/check-output-contract.sh
scripts/ci/check-deterministic-output.sh
scripts/ci/check-report-semantics.sh
scripts/ci/check-release-surfaces.sh
scripts/ci/check-docs-commands.sh
```

For the full local release gate, including Rust fmt/clippy/test/build, release-binary contract scripts, release surfaces, Homebrew formula syntax, real-data CLI smoke, and Rust TUI real-data smoke, run:

```bash
scripts/ci/check-rust-release-local.sh
```

These checks cover:

- demo JSON, Markdown, HTML, and doctor smoke output
- metadata-only session search text and JSON output
- local baseline comparison JSON contract and deterministic fields
- `-o` stdout/stderr behavior and failing gate exit code `2`
- repeated demo latest/overview JSON determinism
- report cost-label and version metadata consistency
- README and Homebrew formula version drift
- non-interactive README/docs command smoke tests
- local real-data CLI smoke with sampled real local session files
- local Rust TUI pty smoke with sampled real local session files

CI uploads generated demo reports as artifacts so reviewers can inspect the JSON, Markdown, and HTML output without rerunning local commands.
