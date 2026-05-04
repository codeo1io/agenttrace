# agenttrace comparison

agenttrace is not another chat UI. It is a local session-history dashboard for people who already use AI coding agents and need to understand cost, token usage, elapsed time, and slow runs across tools.

## What agenttrace is for

Use agenttrace when you want to answer:

- Which agent sessions spent the most cost, tokens, and time?
- Which runs were slow because of long gaps, slow tools, retry loops, large parameters, or context pressure?
- Which session should I inspect first?
- Can I generate local JSON, Markdown, or HTML evidence without uploading private prompts and code?

## Alternatives and adjacent tools

| Option | Best for | Gap agenttrace fills |
|---|---|---|
| Raw JSONL / local log files | Exact source-of-truth inspection | Aggregates many files into ranked sessions, costs, health, latency, and diagnostics |
| IDE or agent chat history | Reading one conversation | Compares sessions across agents, models, costs, tokens, and slow-run signals |
| Token counters / cost-only scripts | Quick spend estimate | Adds time, latency, tool failures, anomalies, context pressure, and TUI triage |
| CI logs | Build outcome evidence | Adds agent-session health gates and self-contained reports |
| Hosted observability tools | Centralized team dashboards | Keeps prompts, code, and logs local by default |

## Feature matrix

| Capability | agenttrace | Raw logs | Agent UI history | Cost-only scripts | Hosted dashboards |
|---|---:|---:|---:|---:|---:|
| Multi-agent local discovery | Yes | Manual | No | Varies | Varies |
| Cost and token totals | Yes | Manual | Varies | Yes | Yes |
| Elapsed time and latency signals | Yes | Manual | Varies | No | Varies |
| Slow tool and hanging-session diagnosis | Yes | Manual | No | No | Varies |
| Context pressure and large parameter hints | Yes | Manual | No | No | Varies |
| Ranked session triage | Yes | No | No | No | Varies |
| Local TUI | Yes | No | No | No | No |
| JSON / Markdown / HTML export | Yes | No | No | Varies | Varies |
| CI health gates | Yes | No | No | Varies | Varies |
| No hosted backend required | Yes | Yes | Yes | Yes | No |

## When not to use agenttrace

agenttrace is probably not the right tool if you need:

- a hosted multi-user SaaS dashboard
- live tracing while an agent is still streaming
- full prompt search across every private log
- billing-grade accounting from provider invoices

It is intentionally a local-first developer tool for post-run inspection.

## Positioning

Short version:

> agenttrace is a local cost explorer and slow-run debugger for AI coding agents.

Longer version:

> agenttrace reads the session files your coding agents already write, ranks the runs that burned cost, tokens, and time, then shows why a task was slow with evidence like long gaps, slow tools, retry loops, large params, and context pressure.
