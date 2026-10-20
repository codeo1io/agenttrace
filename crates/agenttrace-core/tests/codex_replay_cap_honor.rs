//! rm-2020 — the codex replay reconciliation must honor the run's
//! named-file read cap (`AGENTTRACE_FILE_CAP_BYTES`).
//!
//! The reconcile pass re-reads every codex rollout from disk at each of
//! its call sites. That read used to be a bare `std::fs::read`: the
//! parse lane's admission bound (rm-451 — a hostile multi-hundred-MiB
//! journal must be refused before the read, not loaded whole) did not
//! hold here. The assess PoC (run d0c2a3dc cycle 2) pinned the
//! warm-cache shape: with `AGENTTRACE_FILE_CAP_BYTES=512` and a corpus
//! whose sessions survive through the session cache, the pass still
//! read both multi-KB rollouts unbounded and rebased the family — the
//! cap was silently bypassed exactly when the cache disarmed the parse
//! lane that would otherwise have refused the files.
//!
//! The scan now takes the same bounded path the parse lane and the
//! discovery content-signature use: a metadata fast-path refusal before
//! any read, and `take(cap + 1)` so a file that GREW between stat and
//! read (the metadata lie) still cannot allocate past one byte over the
//! cap. An over-cap rollout is a TYPED refusal, distinct from the
//! rm-1542 unreadable arm: the session keeps its full (cache-provided,
//! possibly double-counted) usage and carries a named
//! `codex_multiagent_reconcile_skipped_cap_exceeded` disclosure. Under
//! the default 256 MiB cap no real rollout is refused and the genuine
//! family reconcile is byte-identical before/after.
//!
//! The env override is mutated in-test, so this suite lives in its own
//! test binary: `file_read_cap_bytes()` reads the environment at scan
//! time, and a cap set for one test must not race the parallel tests
//! of the main replay battery.

use agenttrace_core::reconcile_codex_multiagent_replays;
use std::path::PathBuf;

/// Set `AGENTTRACE_FILE_CAP_BYTES` for a scope and restore the exact
/// previous state (absent stays absent) on drop — the same contract
/// `count_honesty_admission`'s inline save/set/restore blocks keep.
struct EnvCapGuard {
    previous: Option<std::ffi::OsString>,
}

impl EnvCapGuard {
    fn set(cap: &str) -> Self {
        let previous = std::env::var_os("AGENTTRACE_FILE_CAP_BYTES");
        std::env::set_var("AGENTTRACE_FILE_CAP_BYTES", cap);
        Self { previous }
    }
}

impl Drop for EnvCapGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var("AGENTTRACE_FILE_CAP_BYTES", value),
            None => std::env::remove_var("AGENTTRACE_FILE_CAP_BYTES"),
        }
    }
}

/// One genuine parent/child family copied to a temp dir, parsed under
/// the default cap — the exact session state a warm cache hands the
/// reconcile after an earlier run admitted the files.
fn warm_family(dir: &std::path::Path) -> Vec<agenttrace_core::Session> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex-multiagent-v2");
    let parent_text = std::fs::read_to_string(base.join("parent.jsonl")).unwrap();
    let child_text = std::fs::read_to_string(base.join("child.jsonl")).unwrap();
    std::fs::write(dir.join("parent.jsonl"), &parent_text).unwrap();
    std::fs::write(dir.join("child.jsonl"), &child_text).unwrap();
    vec![
        agenttrace_core::parse_file(&dir.join("parent.jsonl")).expect("parent parses"),
        agenttrace_core::parse_file(&dir.join("child.jsonl")).expect("child parses"),
    ]
}

#[test]
fn reconcile_honors_the_read_cap_and_default_cap_parity() {
    // Both fixtures are multi-KB; a 512-byte cap refuses each at the
    // metadata fast path.
    let dir = std::env::temp_dir().join(format!("agenttrace-rm2020-cap-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");

    // Warm-cache shape: sessions parsed under the default cap, then the
    // cap lowered below the files' size before the reconcile — the
    // state the assess PoC pinned (the rebase used to happen anyway).
    let mut sessions = warm_family(&dir);
    let naive: Vec<i64> = sessions.iter().map(|s| s.metrics.tokens_input).collect();
    assert_eq!(naive, vec![1_400, 1_700], "PoC shape: naive family totals");
    {
        let _cap = EnvCapGuard::set("512");
        let stats = reconcile_codex_multiagent_replays(&mut sessions);
        assert_eq!(stats.families, 0, "no readable member, no family");
        assert_eq!(stats.children_adjusted, 0, "the rebase must not happen");
        assert_eq!(stats.rollouts_cap_refused, 2, "typed refusal per member");
        assert_eq!(
            stats.rollouts_unreadable, 0,
            "an over-cap file is not unreadable — the arms stay distinct"
        );
        let inputs: Vec<i64> = sessions.iter().map(|s| s.metrics.tokens_input).collect();
        assert_eq!(inputs, naive, "usage kept whole (conservative)");
        for session in &sessions {
            assert_eq!(
                session
                    .metrics
                    .disclosure_counters
                    .get("codex_multiagent_reconcile_skipped_cap_exceeded"),
                Some(&1),
                "each over-cap rollout discloses the typed skip"
            );
            assert!(
                !session
                    .metrics
                    .disclosure_counters
                    .contains_key("codex_multiagent_reconcile_skipped_unreadable"),
                "the cap arm must not wear the unreadable label"
            );
            assert!(
                !session
                    .metrics
                    .disclosure_counters
                    .contains_key("codex_multiagent_replayed_child_adjusted"),
                "no child was adjusted"
            );
        }
    }

    // Default-cap parity: the same corpus, no override — the genuine
    // family must reconcile exactly as it did before rm-2020 (zero
    // behavior change under the default cap).
    let mut control = warm_family(&dir);
    let stats = reconcile_codex_multiagent_replays(&mut control);
    assert_eq!(stats.families, 1);
    assert_eq!(stats.children_adjusted, 1);
    assert_eq!(stats.rollouts_cap_refused, 0);
    assert_eq!(stats.rollouts_unreadable, 0);
    assert_eq!(stats.tokens_input_dropped, 1_400);
    let inputs: Vec<i64> = control.iter().map(|s| s.metrics.tokens_input).collect();
    assert_eq!(inputs, vec![1_400, 300], "the family still reconciles");
    std::fs::remove_dir_all(&dir).ok();
}
