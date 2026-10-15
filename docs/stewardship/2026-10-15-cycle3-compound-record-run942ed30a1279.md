# Compound record — cycle 3, run 942ed30a12794090bac069d8dd852256 (repository-maintenance 1f97144ee1224c44be4bfc0dd980adf7)

- compound attempt: d187c26981e649f5b40abb44e8646c19 (2026-10-15), after full_tests 5edf82aa897349aeb7286872e1f4bac6
- base pin: c0da781 (worktree run-942ed30a1279, HEAD unmoved; porcelain at compound entry = the 4-entry implement handoff)
- contract: PRE-REVIEW compound — consumes ONLY pre-review cycle evidence (assessment, research, roadmap, prioritization, implementation, and test outcomes); NO test executed at compound (phase contract, zero invocations of pytest / run_tests.sh / run_tests_parallel.py / cargo test this turn)
- batch: `ingest truth: one conversation one session, bounded reads` — LEAD rm-1417 (correctness 72.0) + rm-1418 (reliability 34.0), selected by prioritize e5f0509f (ADOPTED), stewarded by 74f8e89c, implemented by ccdd599a
- cycle mechanism for reaped attempts (used THREE times this cycle — each verified before adoption, per the fleet rule):
  - roadmap 60e0cbcb — serve-hold reap at 1794.7s of an 1800s budget, AFTER writing its typed artifact (scratch mtimes 07:18Z vs reap 07:20:38Z); adopted at 6fc33165
  - prioritize e5f0509f — 429 reap after writing its typed artifact + selected-batch.md (md5 5e6eb20f, durable in ~/.cache/at-prioritize-e5f0509f); adopted at 1ea0acb9
  - compound ba95c59a — delivery reap after ~14 min: 16 event-log messages ending `delegate_turn_completed status=succeeded` behind two `phase_result_repair` turns, engine results/ detail `The cached wall mutated between probes (cache-dir sweep corrupted it). Hardening: regenerate from git and build in ONE atomic invocation, spool-side outputs only:` (truncated mid-sentence), typed artifact ABSENT, done/ record carries NO phase_result — this redo adopted its surviving scratch (build_band.py + wall snapshot wall-origin-445450b.md) as the rebuild substrate after live verification

## Changed artifacts this phase

- ROADMAP.md IN-TREE (+36/-0, 14 hunks, pure insertion; postimage md5 bfe82813e8bd5c05970bc085eb2a8263): 2 banners at the banner head (compound c3 + rebuilt cycle-3 roadmap, newest-first), 12 dated evidence riders appended as the last line of each owning row's block, the wall-foot mint band rm-1417/rm-1418 below rm-1065 with the mint comment, and the reconstruction note above the managed footer. Status accounting: 388 def rows in → 390 out, def-max rm-1369 → rm-1418, zero pre-existing statuses flipped, done-flips reserved to the commit gate (rm-012).
- this record (in-tree untracked, the 963662941/38b020c7 doc convention)
- spool scratch `delegate/d187c26981e649f5b40abb44e8646c19-scratch/`: build_compound.py (hardened one-shot builder), ROADMAP.postimage.md, ROADMAP.patch (14 hunks), wall-pin-c0da781.md, compound-record mirror, verification.md
- the worktree LEFT DIRTY per policy (implement batch + this band) — the commit gate owns landing

## Band verification legs (all live this attempt)

1. wall regenerated from git in the same invocation that builds the postimage (`git show HEAD:ROADMAP.md`, md5 asserted 60d023e3af3e79a04e66db85298bfea0 == live file byte-identical) — the prior attempt's dying hardening note, executed
2. postimage md5 bfe82813; 388 → 390 def rows; 12/12 riders verified IN-BLOCK as each block's last non-empty line (python proof); banners at the head; managed footer still the last line; wall is a strict subsequence of the postimage (pure insertion, zero modifications)
3. ROADMAP.patch round-trip on pristine copies BOTH ways: `git apply` → bfe82813 AND `patch --fuzz=0 -p1` → bfe82813 (== postimage)
4. live wall probe: origin/master moved pin+1(8bc0587) → 16dcc9a0 BUT ROADMAP byte-flat vs the prior attempt's 445450b snapshot (both md5 1f1ab384…, 2,575 lines, 395 rows, def-max rm-1502) — rm-1417/rm-1418 zero collisions on the live wall
5. tree census at dispatch: HEAD c0da781, porcelain = the 4 implement entries, git diff md5 967341f2… == implement's recorded batch.diff (handoff intact through full_tests)

## Recorded validation outcomes (consumed as recorded, NOT re-run — compound contract)

- targeted ca163cbf: ingest_truth_batch 5/5 + discovery_contract 104/0 + core --lib 377/0 + `cargo fmt --check` rc0 + `cargo clippy -D warnings` rc0; live-diff md5 == 967341f2 (zero executable delta that turn)
- full 5edf82aa (ci.yml-verbatim — dispatch full_command empty, ci.yml lanes executed verbatim as authority): fmt rc0; clippy --locked (core/tui/agenttrace) rc0; 14 cheap gates rc0 (bash -n/sh -n/ruby -c set, cargo-manifests, release-surfaces, locked-cargo, install-ref-drift, changelog-structure, npm-files, release-pins, 8 repo-contract); core 50 suites 817/0 + tui 60/0 + cli 236/0 (1,113 total vs the assess baseline 1,108 = the 5 new contract arms); MSRV +1.89 check rc0 (1m29s); token_conformance 2/0; conformance runner 13 cases 1-xfail rc0; npm 4/4
- LIVE PoCs from implement (release bin): the assess mirror corpus `.gemini/tmp/<root>/{chats,checkpoints}` renders `1 Sessions` (assess recorded `planning 2 Sessions` — rm-1417 flipped); the /opencode/storage corpus with an 8.4 MiB msg_assistant.json renders `1 Sessions 100% Healthy` with the record skipped + disclosed (rm-1418)

## Standing reds (pre-existing, delta-untouched, owned elsewhere — disclosed, not bundled)

- check-install-runtime test D: install-runtime.sh:152 pins `^ref=v0.9.0$` vs install.sh:13 `REF=v0.10.1` — the rm-051 arc; sibling-owned lane, reconcile by title
- cargo deny RUSTSEC-2026-0346..0349 x4 (rustls 0.23.45): advisory-DB fetch-time drift, equally red at origin; fix = the sibling rustls 0.23.46 Cargo.lock bump — do NOT bundle into this batch

## Reusable lessons (this cycle)

- L1 (reap economics): three of this cycle's phases survived ONLY as typed artifacts after transport reaps (serve-hold 1794.7/1800s; 429; delivery). Writing the typed envelope + a durable artifact BEFORE polish is what makes attempts adoptable. The adopt-verify-redo loop (legs: identity, tree census, worktree cleanliness at dispatch, live seam re-probe) is now routine and cheap.
- L2 (spool/tmp sweep reality): the spool reaper eventually takes scratch dirs too — the roadmap phase's ENTIRE scratch (patch + postimage + wall-base) and every /tmp mirror vanished before compound. Durable mirrors belong under ~/.cache/at-* (outside repos, sdb) AND in-tree-uncommitted-once-compound-owns-the-band (the 2736ab2f-era convention that replaced the older spool-side-only delivery of 4401a380).
- L3 (cached-wall corruption): a cached wall file mutated between probes (cache-dir sweep). Regenerate the wall from git and build the postimage in ONE atomic invocation with md5 asserts at entry — never trust a wall copy that sat in a sweepable location across tool calls.
- L4 (mint-vs-landing races): e4725315 minted rm-1500..rm-1502 on master BETWEEN this run's mint decision (rm-1417/1418, fog ceiling then rm-1434) and this compound rebuild. Numerals are fleet-ledgered, never renumbered pre-landing (rm-708); the wall-lag reconciles by title at the commit gate. Fog ceilings age in hours on this fleet — every mint needs a fresh live census.
- L5 (multi-line insert trap, builder bug caught this turn): inserting a multi-line python string as ONE list element into a line-model silently defeats per-line regex scans and md5-still-matches-nothing assertions; split to `str.split("\n")` before insert. Symmetric to the #18446 -U0 relocation lesson: verify EVERY stage by content (in-block rider proof, subsequence purity), never by exit code alone.
- L6 (selection discipline over triple-claimed lanes): rm-1053 had THREE live uncommitted sibling builds at prioritize time (7bb862e4 raw-first / f2fccaa9 resolution-ladder / bbf61fbd exact-id-first); the cycle deliberately ceded it and took the unclaimed correctness seam (rm-1417) instead. Conflict-avoidance by dirty-tree census beat racing a crowded lane.

## Prevention rules

- P1: any phase that produces a band/patch must round-trip it BOTH ways (git apply AND patch --fuzz=0) on pristine copies before declaring it applied — exit code alone is not proof (the fd610df5 -U0 relocation and this turn's `git apply` silently-not-applying both prove it).
- P2: mint decisions MUST record the fog census inputs (swept surface list + ceiling id) in the mint comment so later collisions reconcile by evidence, not memory.
- P3: a compound that consumes recorded outcomes must re-verify the handoff intactness (diff md5 + porcelain census) at entry — a mutated handoff invalidates every recorded outcome downstream.
- P4: pinned-ref install tests (rm-051 class) drift red the moment the shipped default ref moves; the cure is pinning the ref in the TEST from the install script's default, or reading it dynamically. Carried for the owning sibling lane.

## Next-cycle context (starters, not commitments)

- rm-1419+ free ONLY after a fresh live census (fog frontier was rm-1434 at this run's mint; e4725315 already pushed the landed wall to rm-1502 — next cycle's census must sweep BOTH the spool and the 10-worktree dirty set)
- natural candidates surfaced this cycle but NOT taken: rm-1053 pricing keying (triple-claimed; adjudicate by content-hunk once siblings land), rm-546 semconv first-tag trigger (0 tags at research time; re-probe), rm-537/rm-553 Cowork ledger+discovery (tokscale #1437 is the design spec), rm-421 autoCompactWindow subagent compaction episodes, rm-992 codex rollout .zst opacity (pin fixtures to the SHIPPED schema when 0.163 lands)
- the standing reds (rm-051 install-ref pin, rustls 0.23.46 bump) remain the cheapest wall-red cures on the fleet

## Commit-gate seams (for final_validation → commit, NOT this phase)

1. land order: the 4-file implement batch (discovery.rs +261, parser.rs +62/-7, NEW tests/ingest_truth_batch.rs, discovery_contract.rs CU-18) + ROADMAP.md band (+36) + this doc; single commit or batched per gate policy
2. base lag: HEAD c0da781 vs origin/master 16dcc9a0 (217783f, 3e90993, 94e1db6 + b5d391ec pricing.rs cost-time/ultrafast, 8bc0587, e4725315/9e84599 ROADMAP rm-1500..1502, 445450b, 16dcc9a0) — reconcile by title; pricing.rs is UNTOUCHED by this batch so the lookup_price_tiered/convert_litellm rewrites land clean
3. ROADMAP: this band applies to the PIN wall (riders anchor by row-id; mint rows rm-1417/1418 anchor at the wall foot below the landed ceiling rm-1502 on the live wall); reconcile with e4725315's already-landed band by title/hunk, never whole-file; duplicate-numeral fog sets {1417x2, 1418x2} exist by census — landed-first wins, never renumber
4. CHANGELOG gap disclosed: the implement batch carries NO Unreleased bullet (every landed maintenance batch carries one — the a93aa56a precedent); add one Fixed bullet for rm-1417/rm-1418 at the gate (changelog-structure was green without it at full_tests; re-run the gate after adding)
5. standing reds stay unbundled: rm-051 pin + rustls 0.23.46 belong to sibling lanes; verify the install-runtime test + cargo deny verdicts against the tree AFTER the lag reconcile, not before

— recorded 2026-10-15 by compound d187c26981e649f5b40abb44e8646c19
