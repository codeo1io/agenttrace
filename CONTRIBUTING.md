# Contributing to agenttrace

Thanks for helping make AI coding-agent observability less opaque.

agenttrace is intentionally small: one Rust binary, local-first analysis, no hosted database, and no background service. It can read local SQLite stores written by supported agents. Changes should keep that shape unless there is a clear reason.

## Development Setup

```bash
git clone https://github.com/luoyuctl/agenttrace.git
cd agenttrace
cargo test
cargo build --release -p agenttrace
target/release/agenttrace --demo
target/release/agenttrace --doctor
```

## Validation Checklist

Run the relevant checks before opening a PR:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test
cargo build --release -p agenttrace
ruby -c homebrew/Formula/agenttrace.rb
```

For TUI changes, also check narrow and wide terminals when possible:

```bash
target/release/agenttrace --demo
target/release/agenttrace --demo --doctor
target/release/agenttrace --demo --overview -f json
```

## Maintenance Records and the Retention Bound

Stewardship records (`docs/stewardship/`) are append-only evidence for the
maintenance campaigns: one file per attempt phase, named
`YYYY-MM-DD-cycle<N>-<phase>-record…md`. They are never edited after the
run that writes them.

The inline directory is not a landfill (`rm-1984`): only the **12
most-recent maintenance cycles** stay under `docs/stewardship/`. When a
new cycle pushes the count past the bound:

1. `git mv docs/stewardship/<file> docs/archive/stewardship/<file>`
   (flat move, original basename — the date prefixes keep names unique);
2. `scripts/ci/check-stewardship-retention.sh --regen-index` rebuilds
   `docs/archive/stewardship/INDEX.md`, which maps every archived record
to its original inline path — cite either form; the index is the bridge;
3. `scripts/ci/check-stewardship-retention.sh` (also a CI gate) must pass.

Historical citations in ROADMAP banners and review/research docs keep
their original inline-path spelling on purpose — that is why the archive
is flat and the index maps both directions.

## Translations

All user-visible text lives in `crates/agenttrace-core/locales/en.yml` and
`zh-CN.yml`. Add a key to both files (keep the same `%{placeholders}`) and look it
up with `tr(language, "area.key")` or `Message::new("area.key").arg(...)`. Do not
inline English/Chinese pairs in code; `scripts/ci/check-i18n.sh` rejects them and
checks that both catalogs stay in sync.

## Parser Contributions

Parser PRs are very welcome. Please read [docs/guides/parser-guide.md](docs/guides/parser-guide.md) first.

A good parser PR includes:

- redacted or synthetic test data
- format detection
- extraction for role, timestamp, model, token usage, tool calls, and tool errors when available
- regression tests for malformed or partial records
- no private prompts, API keys, file paths, or proprietary source snippets

Do not make a fixture look more complete than the source format. Aggregate-only
sources should remain Aggregate or Limited instead of receiving synthetic event
timestamps or fake tool spans.

Agent-run parser, growth, release, or quality review work should also follow
[docs/maintainers/agentops-prompt-rules.md](docs/maintainers/agentops-prompt-rules.md) before opening
or approving a PR.

## Privacy

Session logs often contain prompts, source paths, repository names, tool arguments, and sometimes secrets. Do not paste raw private logs into issues or PRs.

Prefer one of these:

- synthetic fixtures that preserve shape but not content
- tiny redacted snippets with secrets removed
- `agenttrace --doctor -f json` output for discovery/cache issues

## Community

Participation in this project is covered by the [Code of Conduct](CODE_OF_CONDUCT.md).

## Project Taste

agenttrace should stay:

- fast to install and run
- useful with zero configuration
- readable in small terminals
- honest about unsupported formats
- boring in the best way: deterministic, local, and easy to debug
