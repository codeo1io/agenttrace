# Distribution guide

This guide describes the committed AgentTrace release surfaces and their owners. It is for maintainers; end users should start with the repository [README](../../README.md).

## Source of truth

A `v*` tag triggers [the release workflow](../../.github/workflows/release.yml). It builds and publishes signed-by-checksum GitHub Release assets for:

- Linux AMD64 and ARM64
- macOS Intel and Apple Silicon
- Windows AMD64 and ARM64

The release workflow also creates `checksums.txt` and provenance attestations. The GitHub Release is the source for the shell and PowerShell installers:

```text
install.sh
install.ps1
```

These scripts remain at repository root because users invoke them through stable raw-GitHub URLs. `install.sh` verifies the downloaded asset against the release's `.sha256` sidecar when both are present — a mismatch aborts the install, a missing sidecar or missing checksum tool only warns. That check protects against corrupted mirrors and asset drift; since the asset and its sidecar share one origin and channel, it is not a defense against a compromised release origin.

## Homebrew

The checked-in `homebrew/Formula/agenttrace.rb` is a `HEAD` Formula used only for local validation. The release workflow generates the versioned, checksum-pinned Formula from the tag and GitHub Release assets, then publishes it to `luoyuctl/homebrew-tap`.

npm and WinGet are also release channels:

- The workflow sets the npm package version from the `v*` tag immediately before packing and publishing it. Its postinstall hook downloads the matching checksummed GitHub Release binary.
- The workflow renders Homebrew and WinGet metadata from the same `checksums.txt` artifact.
- WinGet submits `Luoyuctl.AgentTrace` through `winget-create`.

The source tree deliberately uses non-release version placeholders. A release tag is the only source of a public version, so package metadata and rendered manifests never need manual version bumps.

## Release checks

Run the local Rust release gate before tagging:

```bash
scripts/ci/check-rust-release-local.sh
```

It validates formatting, linting, tests, build output, parser fixtures, report contracts, release surfaces, Homebrew syntax, helper scripts, and real-data smoke paths.

For public-facing changes, follow the protected-surface rules in [AgentOps prompt rules](agentops-prompt-rules.md).
