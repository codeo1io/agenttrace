# agenttrace bash completion (rm-088)
# Word list generated from `agenttrace --help`; refresh when flags change.
# Install: agenttrace --completions >> ~/.bashrc  (or source directly)
_agenttrace_complete() {
    local cur
    cur="${COMP_WORDS[COMP_CWORD]}"
    local opts="--anomaly --audit --baseline --baseline-max-cost-delta-pct --baseline-max-duration-delta-pct --baseline-max-token-delta-pct --budget --card-theme --clear-cache --compare --completions --config --context-trends --cost --delivery-evidence --demo --diagnostics --dir --doctor --fail-on-critical --fail-under-health --fetch --format --health --help --history-dir --include-history --inspect --lang --latest --limit --list-models --max-tool-fail-rate --mcp-governance --model-filter --no-baseline-gate --order --overview --preserve-history --pricing-file --project --query --range --recommend --sample --search --search-limit --sessions --sort --source --statusline-report --storage --test-match --update-pricing --version --waste --weekly-budget"
    COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
    return 0
}
complete -F _agenttrace_complete agenttrace
