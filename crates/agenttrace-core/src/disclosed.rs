//! Disclosed skip registry (rm-393 discovery skip class, rm-396 parser
//! size bound): the twin "corpus silently shrank" defects are fixed by
//! recording every skip where it happens and disclosing it wherever the
//! corpus is summarized, instead of letting the shrink be invisible.
//!
//! Skip classes are recorded globally because the layers that skip
//! (directory walks, parse workers on background threads) are far from
//! the surfaces that disclose (`--doctor`, report actions in the CLI).
//! The registry is process-wide and append-only; each disclosure site
//! takes a `snapshot` for its own report. Tests must scope assertions
//! to their own paths because tests run in parallel in one process.

use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DisclosedSkipKind {
    /// A directory the session walk could not read (permissions, I/O
    /// error): every session under it is invisible to the run (rm-393).
    UnreadableDirectory,
    /// A session file above the parser's size bound, skipped instead of
    /// slurped (rm-396). `detail` carries the recorded size.
    OversizedFile,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DisclosedSkip {
    pub kind: DisclosedSkipKind,
    pub path: String,
    /// Class-specific context: the OS error text for unreadable
    /// directories, the byte size for oversized files.
    pub detail: String,
}

impl DisclosedSkipKind {
    /// Human label for the `--doctor` text disclosure.
    pub fn label(self) -> &'static str {
        match self {
            DisclosedSkipKind::UnreadableDirectory => "unreadable directory",
            DisclosedSkipKind::OversizedFile => "oversized file",
        }
    }
}

static SKIPS: Mutex<Vec<DisclosedSkip>> = Mutex::new(Vec::new());

/// Records a skip with the given detail line.
pub fn record(kind: DisclosedSkipKind, path: &Path, detail: String) {
    let mut skips = SKIPS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // Keep the registry bounded in pathological scans: the first 256
    // skips carry the disclosure; beyond that the count in the summary
    // keeps telling the truth without growing without limit.
    if skips.len() < 256 {
        skips.push(DisclosedSkip {
            kind,
            path: path.to_string_lossy().to_string(),
            detail,
        });
    }
}

/// Records a skip whose detail is the OS error (read failures).
pub fn record_io_error(kind: DisclosedSkipKind, path: &Path, error: &std::io::Error) {
    record(kind, path, error.to_string());
}

/// A point-in-time copy of everything recorded so far this process.
pub fn snapshot() -> Vec<DisclosedSkip> {
    SKIPS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// Drops everything recorded so far (test isolation).
pub fn reset() {
    SKIPS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_snapshot_round_trip_is_path_scoped() {
        // The registry is process-global and tests run in parallel, so
        // assertions stay scoped to a path unique to this test (no
        // reset(): clearing would race other tests' records).
        let unique = format!("/tmp/at-disclosed-{}", std::process::id());
        record(
            DisclosedSkipKind::OversizedFile,
            Path::new(&unique),
            "999 bytes above the 512 MiB parse bound".to_string(),
        );
        let snapshot = snapshot();
        let mine = snapshot
            .iter()
            .find(|skip| skip.path == unique)
            .expect("recorded skip must appear in the snapshot");
        assert_eq!(mine.detail, "999 bytes above the 512 MiB parse bound");
    }
}
