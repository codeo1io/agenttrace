#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "check-readme-zh-parity: $*" >&2
  exit 1
}

# rm-2283 (run 940e1aab cycle 4, "doc-truth gates" rider): every
# English README section must be EITHER mirrored in README.zh-CN.md
# (translated title, recorded in the alias ledger below) OR explicitly
# ceded to the English README (recorded in the cede ledger below AND
# named verbatim inside README.zh-CN.md's 未翻译章节 cession block, so
# a zh reader can always find it). Pre-gate state (assessed
# 2026-10-22 at 6b8c5c0a): fenced-code-aware census EN 25 headings vs
# zh 12 — 13 EN sections had no zh counterpart and no cession record;
# a naive grep census miscounted because both READMEs carry '#'
# comment lines inside fenced code blocks. This gate is fenced-aware.
#
# Red-first proof (rm-2283): on the pre-port tree this script fails
# naming all 13 uncovered EN sections (verified 2026-10-22 by running
# it before the zh ports landed); on the ported+ceded tree it passes.
# A NEW English section now forces a conscious choice — port it, or
# add it to the cession ledger — instead of silent drift.

repo_root="$(git rev-parse --show-toplevel)" \
  || fail "must run inside the agenttrace repository"

cd "$repo_root"

en="README.md"
zh="README.zh-CN.md"
[[ -f "$en" && -f "$zh" ]] || fail "missing $en or $zh"

# Fenced-code-aware markdown heading extraction: '```' toggles the
# fence; '#' lines inside fences are shell/TOML comments, not
# headings. Emits 'level<TAB>title'.
headings() {
  awk '
    /^```/ { infence = !infence; next }
    infence { next }
    /^(##|###) / {
      line = $0
      sub(/^#+ /, "", line)
      match($0, /^#+/)
      print substr($0, 1, RLENGTH) "\t" line
    }
  ' "$1"
}

# Alias ledger: EN title <TAB> zh title. Pairs with a deliberate
# structural divergence are fine — the ledger matches by TITLE, not
# by level (zh keeps 自动发现的会话根目录 at ## although EN nests it
# under ## CLI flag reference, which is ceded).
declare -A alias_of
while IFS=$'\t' read -r en_title zh_title; do
  [[ -z "$en_title" ]] && continue
  alias_of["$en_title"]="$zh_title"
done <<'LEDGER'
Why agenttrace?	为什么需要 agenttrace？
Real local run	真实本机运行
Install	安装
Update	更新
Quickstart	Quickstart
Flags go before the session path	标志写在会话路径之前
Local MCP server (rm-455)	本地 MCP 服务器（rm-455）
Auto-discovered session roots	自动发现的会话根目录
What you get	你会得到什么
Docs	文档
Contributing	贡献
License	许可证
Pricing	定价
Reading a session from stdin	从 stdin 读取会话
Named session files are bounded too	命名会话文件同样有上限
`--baseline` and `--compare`	`--baseline` 与 `--compare`
Shareable usage card (SVG)	可分享的用量卡片（SVG）
LEDGER

# Cede ledger: EN sections intentionally left in English only. Each
# must ALSO be named verbatim in README.zh-CN.md (the 未翻译章节
# cession block), or the cession itself has drifted.
ceded=(
  'Configuration file'
  'CSV statement export (rm-409)'
  'Governance reports'
  'Compressed (zstd) Codex rollouts'
  '`--demo` never silently replaces an explicit session source'
  'Quota view'
  'Statusline capture'
  'CLI flag reference'
)

# zh headings with no EN counterpart (the cession block itself).
zh_only=(
  "未翻译章节"
)

declare -A zh_titles
while IFS=$'\t' read -r _lvl title; do
  zh_titles["$title"]=1
done < <(headings "$zh")

uncovered=""
cession_drift=""
en_count=0
while IFS=$'\t' read -r _lvl en_title; do
  en_count=$((en_count + 1))
  expect="${alias_of[$en_title]:-$en_title}"
  if [[ -n "${zh_titles[$expect]:-}" ]]; then
    continue
  fi
  is_ceded=0
  for c in "${ceded[@]}"; do
    [[ "$c" == "$en_title" ]] && is_ceded=1
  done
  if [[ $is_ceded -eq 1 ]]; then
    grep -qF "$en_title" "$zh" || cession_drift+="$en_title"$'\n'
    continue
  fi
  uncovered+="$en_title"$'\n'
done < <(headings "$en")

report=""
[[ -z "$uncovered" ]] \
  || report+="EN README sections with no zh port and no cession record:"$'\n'"$uncovered"
[[ -z "$cession_drift" ]] \
  || report+="ceded sections not named verbatim in README.zh-CN.md (cession block drift):"$'\n'"$cession_drift"
[[ -z "$report" ]] || fail "$report"

# Reverse direction: a zh heading must be an alias target, a cede
# record, or an explicit zh-only entry — no orphan zh sections.
orphans=""
while IFS=$'\t' read -r _lvl zh_title; do
  covered=0
  for en_title in "${!alias_of[@]}"; do
    [[ "${alias_of[$en_title]}" == "$zh_title" ]] && covered=1
  done
  for c in "${ceded[@]}"; do
    [[ "$c" == "$zh_title" ]] && covered=1
  done
  for o in "${zh_only[@]}"; do
    [[ "$o" == "$zh_title" ]] && covered=1
  done
  [[ $covered -eq 1 ]] || orphans+="$zh_title"$'\n'
done < <(headings "$zh")

[[ -z "$orphans" ]] || fail "zh README headings with no EN counterpart:"$'\n'"$orphans"

echo "check-readme-zh-parity: OK — all $en_count EN sections ported or explicitly ceded"
