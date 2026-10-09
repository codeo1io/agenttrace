#compdef agenttrace
# agenttrace zsh completion (rm-088)
# Installed with: agenttrace --completions >> ~/.zshrc  (after compinit)
_agenttrace() {
    local -a opts
    opts=(
        '--overview[render the global overview]'
        '--sessions[render the session list]'
        '--diagnostics[render diagnostics]'
        '--waste[render the waste report]'
        '--recommend[render recommendations]'
        '--compare[run the baseline compare gate]'
        '--doctor[run the environment doctor]'
        '--storage[print the read-only storage footprint report]'
        '--search:pattern[full-text search transcripts]'
        '--list-models[list pricing-catalog models]'
        '--completions[print shell completion snippets]'
        '--version[print the binary version]'
        '--dir:DIR[session directory to scan]'
        '--format:FORMAT[output format: text|json|csv|html|svg]'
        '--help[print help]'
    )
    _arguments -S $opts
}
_agenttrace "$@"
