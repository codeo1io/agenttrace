# Privacy

agenttrace is local-first. It reads AI coding-agent session logs from paths you choose, computes metrics on your machine, and does not upload prompts, code, logs, reports, or telemetry to any hosted service.

Generated reports are written only to the output path you request with `-o`. Review reports before sharing them, because they can contain filenames, command names, model names, token counts, costs, and excerpts derived from local session logs.

agenttrace runs fully offline by default. Reports use a dated pricing snapshot bundled with the binary, and no report or test path contacts the network. The only exception is `agenttrace --update-pricing`, which downloads public model pricing metadata from the LiteLLM community pricing source and caches it locally for later runs. It does not send your local session logs.
