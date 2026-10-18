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

## CI and Dependency Automation

`.github/dependabot.yml` declares weekly `cargo` and `github-actions`
update groups, but on this fork that config is currently
declared-but-dead: Dependabot has never opened a PR here (as of
2026-10-13, `gh api search/issues -f q=repo:codeo1io/agenttrace+type:pr+author:app/dependabot`
returns `total_count: 0`) while the upstream twin config keeps firing
(luoyuctl/agenttrace PR #319). The most likely cause is the fork's
repository-settings Dependabot toggle being off — enabling it is a
GitHub-ops action (Settings → Code security → Dependabot version
updates), not a change to any file in this tree. Verification path:
after the toggle is flipped, confirm the first wave lands at the next
weekly window (Mondays) with the search above returning PRs; until
then, action-pin bumps land by hand, mirroring the upstream
Dependabot PRs (they run green upstream) plus fork-only pins resolved
from their tags (rm-1001; the 2026-10-13 wave mirrors upstream #319
and bumps scorecard-action to v2.5.0).

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
