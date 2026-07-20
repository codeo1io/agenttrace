# WinGet manifest generation

WinGet manifests are generated from the release checksums by:

```bash
scripts/release/render-channels.sh <version> <checksums.txt> <output-dir>
```

The release workflow submits the generated files to `microsoft/winget-pkgs` as
`Luoyuctl.AgentTrace`. The initial submission requires the `WINGET_GITHUB_TOKEN`
repository secret; subsequent releases use the same path.
