//! Read-only storage-footprint report (rm-692).
//!
//! `agenttrace --storage` answers "what is my agent-data footprint on disk?"
//! WITHOUT parsing sessions or opening databases: per-provider file counts
//! and logical bytes, the ten largest source files, the session-cache size,
//! and available space per filesystem. The walk is read-only — SQLite
//! WAL/SHM sidecars are counted by name and never opened; root symlinks are
//! resolved exactly once; nested symlinks and inaccessible paths are
//! disclosed as skipped, and the report names its coverage as `partial`
//! whenever anything was skipped. External spec: codeburn #1626 / PR #1686
//! (`codeburn storage`), corroborated by this repository's roadmap row
//! rm-692; implemented fresh here (no shared code with the doctor's
//! dir-report lane, per the stewardship boundary).

use crate::discovery::{known_session_dirs, KnownSessionDir};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// One provider storage root (e.g. `~/.claude/projects`) with its footprint.
#[derive(Debug, Serialize)]
pub struct StorageProvider {
    /// Provider name as discovery knows it (e.g. `claude`).
    pub name: String,
    /// Root as configured/known (pre-resolution).
    pub root: PathBuf,
    /// Root after resolving symlinks exactly once (canonical path).
    pub resolved_root: PathBuf,
    /// Number of regular files under the root (sidecars included).
    pub files: u64,
    /// Logical (apparent) size in bytes of those files.
    pub logical_bytes: u64,
    /// Count of SQLite WAL/SHM sidecar files (counted, never opened).
    pub sqlite_sidecars: u64,
    /// Logical bytes of the sidecar files.
    pub sqlite_sidecar_bytes: u64,
}

/// One of the ten largest single files across all roots.
#[derive(Debug, Serialize)]
pub struct LargeFile {
    pub path: PathBuf,
    pub bytes: u64,
}

/// Session-cache directory footprint.
#[derive(Debug, Serialize)]
pub struct CacheFootprint {
    pub dir: PathBuf,
    pub exists: bool,
    pub files: u64,
    pub bytes: u64,
}

/// Available space on the filesystem holding the given root.
#[derive(Debug, Serialize)]
pub struct FilesystemSpace {
    pub root: PathBuf,
    /// `None` when statfs is unavailable on this platform/path.
    pub available_bytes: Option<u64>,
}

/// A path skipped during the walk, with the reason disclosed.
#[derive(Debug, Serialize)]
pub struct SkippedPath {
    pub path: PathBuf,
    pub reason: String,
}

/// The full storage report rendered by `agenttrace --storage`.
#[derive(Debug, Serialize)]
pub struct StorageReport {
    pub providers: Vec<StorageProvider>,
    pub ten_largest: Vec<LargeFile>,
    pub cache: CacheFootprint,
    pub filesystems: Vec<FilesystemSpace>,
    pub skipped: Vec<SkippedPath>,
    /// `full` when nothing was skipped, `partial` otherwise.
    pub coverage: &'static str,
    /// Human-facing notes (dedup decisions, empty roots, platform notes).
    pub notes: Vec<String>,
}

fn is_sqlite_sidecar(name: &str) -> bool {
    name.ends_with("-wal") || name.ends_with("-shm")
}

/// Available bytes on the filesystem containing `root` via statfs(2).
#[cfg(unix)]
fn available_bytes(root: &Path) -> Option<u64> {
    let cpath = std::ffi::CString::new(root.as_os_str().as_encoded_bytes()).ok()?;
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    // SAFETY: `stat` is a valid, zeroed out-param and `cpath` is a valid
    // NUL-terminated path buffer; statfs only writes to `stat`.
    let rc = unsafe { libc::statfs(cpath.as_ptr(), &mut stat) };
    if rc != 0 {
        return None;
    }
    let block_size = stat.f_bsize.max(0) as u64;
    Some(block_size.saturating_mul(stat.f_bavail))
}

#[cfg(not(unix))]
fn available_bytes(_root: &Path) -> Option<u64> {
    None
}

struct WalkTotals {
    files: u64,
    logical_bytes: u64,
    sidecars: u64,
    sidecar_bytes: u64,
    largest: Vec<LargeFile>,
}

impl WalkTotals {
    fn note_file(&mut self, path: PathBuf, bytes: u64) {
        self.files += 1;
        self.logical_bytes += bytes;
        if is_sqlite_sidecar(
            path.file_name()
                .map(|n| n.to_string_lossy().as_ref().to_owned())
                .unwrap_or_default()
                .as_str(),
        ) {
            self.sidecars += 1;
            self.sidecar_bytes += bytes;
        }
        // Maintain a top-10 by insertion with a small sort on push.
        self.largest.push(LargeFile { path, bytes });
        if self.largest.len() > 64 {
            self.largest
                .sort_by_key(|large| std::cmp::Reverse(large.bytes));
            self.largest.truncate(10);
        }
    }
}

/// Recursively walk `dir`, counting regular files. Nested symlinked
/// directories are skipped (disclosed by the caller); root symlinks are
/// resolved once by the caller before the walk starts.
fn walk_dir(dir: &Path, totals: &mut WalkTotals, skipped: &mut Vec<SkippedPath>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            skipped.push(SkippedPath {
                path: dir.to_path_buf(),
                reason: format!("unreadable: {err}"),
            });
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(err) => {
                skipped.push(SkippedPath {
                    path: path.clone(),
                    reason: format!("stat failed: {err}"),
                });
                continue;
            }
        };
        if meta.file_type().is_symlink() {
            // Symlinked files/dirs inside a root: resolve files for size,
            // skip nested directory symlinks (loop safety).
            match fs::metadata(&path) {
                Ok(target) if target.is_file() => {
                    totals.note_file(path, target.len());
                }
                Ok(_) => skipped.push(SkippedPath {
                    path,
                    reason: "nested directory symlink skipped".to_string(),
                }),
                Err(err) => skipped.push(SkippedPath {
                    path,
                    reason: format!("dangling symlink: {err}"),
                }),
            }
            continue;
        }
        if meta.is_dir() {
            walk_dir(&path, totals, skipped);
        } else if meta.is_file() {
            let bytes = meta.len() as u64;
            totals.note_file(path, bytes);
        }
    }
}

/// Build the read-only storage report over the known provider roots and the
/// session cache directory.
pub fn storage_report(cache_dir: &Path) -> StorageReport {
    storage_report_with_roots(&known_session_dirs(), cache_dir)
}

/// `storage_report` over an explicit root set — the deterministic seam the
/// synthetic smoke fixtures drive (`tests/storage_report_smoke.rs`);
/// identical behavior, discovery merely pre-resolved.
pub fn storage_report_with_roots(roots: &[KnownSessionDir], cache_dir: &Path) -> StorageReport {
    let mut providers = Vec::new();
    let mut skipped: Vec<SkippedPath> = Vec::new();
    let mut totals = WalkTotals {
        files: 0,
        logical_bytes: 0,
        sidecars: 0,
        sidecar_bytes: 0,
        largest: Vec::new(),
    };
    let mut notes: Vec<String> = Vec::new();
    let mut walked_roots: BTreeSet<PathBuf> = BTreeSet::new();

    for dir in roots {
        let resolved = match fs::canonicalize(&dir.path) {
            Ok(resolved) => resolved,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                // Missing roots are normal (provider not installed) — no
                // skip entry, just an absent provider row.
                continue;
            }
            Err(err) => {
                skipped.push(SkippedPath {
                    path: dir.path.clone(),
                    reason: format!("root unresolvable: {err}"),
                });
                continue;
            }
        };
        if !walked_roots.insert(resolved.clone()) {
            notes.push(format!(
                "provider root {} ({}) canonicalizes into an already-walked root — deduped",
                dir.path.display(),
                dir.name
            ));
            continue;
        }
        let mut provider_totals = WalkTotals {
            files: 0,
            logical_bytes: 0,
            sidecars: 0,
            sidecar_bytes: 0,
            largest: Vec::new(),
        };
        walk_dir(&resolved, &mut provider_totals, &mut skipped);
        for large in provider_totals.largest.drain(..) {
            totals.largest.push(large);
        }
        totals.files += provider_totals.files;
        totals.logical_bytes += provider_totals.logical_bytes;
        totals.sidecars += provider_totals.sidecars;
        totals.sidecar_bytes += provider_totals.sidecar_bytes;
        providers.push(StorageProvider {
            name: dir.name.clone(),
            root: dir.path.clone(),
            resolved_root: resolved,
            files: provider_totals.files,
            logical_bytes: provider_totals.logical_bytes,
            sqlite_sidecars: provider_totals.sidecars,
            sqlite_sidecar_bytes: provider_totals.sidecar_bytes,
        });
    }
    if providers.is_empty() {
        notes.push("no provider roots found — nothing discovered on this host".to_string());
    }

    // Session-cache footprint (0600 cache dir; counted, never mutated).
    let mut cache_files = 0u64;
    let mut cache_bytes = 0u64;
    let cache_exists = cache_dir.is_dir();
    if cache_exists {
        let mut cache_totals = WalkTotals {
            files: 0,
            logical_bytes: 0,
            sidecars: 0,
            sidecar_bytes: 0,
            largest: Vec::new(),
        };
        walk_dir(cache_dir, &mut cache_totals, &mut skipped);
        cache_files = cache_totals.files;
        cache_bytes = cache_totals.logical_bytes;
    }

    totals
        .largest
        .sort_by_key(|large| std::cmp::Reverse(large.bytes));
    totals.largest.truncate(10);

    // Available space per DISTINCT filesystem among roots + cache dir.
    let mut filesystems = Vec::new();
    let mut seen_fs: BTreeSet<u64> = BTreeSet::new();
    let mut fs_roots: Vec<PathBuf> = providers.iter().map(|p| p.resolved_root.clone()).collect();
    fs_roots.push(cache_dir.to_path_buf());
    for root in fs_roots {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if let Ok(meta) = fs::metadata(&root) {
                if !seen_fs.insert(meta.dev()) {
                    continue;
                }
            }
        }
        filesystems.push(FilesystemSpace {
            root: root.clone(),
            available_bytes: available_bytes(&root),
        });
    }

    let coverage = if skipped.is_empty() {
        "full"
    } else {
        "partial"
    };
    if coverage == "partial" {
        notes.push(format!(
            "coverage is partial: {} path(s) skipped — see `skipped`",
            skipped.len()
        ));
    }

    StorageReport {
        providers,
        ten_largest: totals.largest,
        cache: CacheFootprint {
            dir: cache_dir.to_path_buf(),
            exists: cache_exists,
            files: cache_files,
            bytes: cache_bytes,
        },
        filesystems,
        skipped,
        coverage,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_detection_by_name_only() {
        assert!(is_sqlite_sidecar("session.db-wal"));
        assert!(is_sqlite_sidecar("opencode.db-shm"));
        assert!(!is_sqlite_sidecar("opencode.db"));
        assert!(!is_sqlite_sidecar("wal-backup.txt"));
    }

    #[test]
    fn report_on_synthetic_root_counts_files_and_sidecars() {
        let tmp = tempfile_dir("rm692-smoke");
        let provider = tmp.join("provider-a");
        std::fs::create_dir_all(provider.join("nested")).unwrap();
        std::fs::write(provider.join("sessions.jsonl"), "line\nline\n").unwrap();
        std::fs::write(provider.join("nested").join("big.jsonl"), "x".repeat(2048)).unwrap();
        std::fs::write(provider.join("opencode.db-wal"), "y".repeat(128)).unwrap();
        std::fs::write(provider.join("opencode.db-shm"), "z".repeat(64)).unwrap();
        let cache = tmp.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("sessions.json"), "{}").unwrap();

        // Point the report at the synthetic tree: monkeypatching discovery
        // is not possible read-only, so drive the walk directly.
        let mut totals = WalkTotals {
            files: 0,
            logical_bytes: 0,
            sidecars: 0,
            sidecar_bytes: 0,
            largest: Vec::new(),
        };
        let mut skipped = Vec::new();
        walk_dir(&provider, &mut totals, &mut skipped);
        assert_eq!(
            totals.files, 4,
            "four regular files under the synthetic root"
        );
        assert_eq!(totals.sidecars, 2, "WAL and SHM counted by name");
        assert_eq!(
            totals.sidecar_bytes,
            128 + 64,
            "sidecar bytes = WAL + SHM logical sizes"
        );
        assert_eq!(totals.logical_bytes, 10 + 2048 + 128 + 64);
        assert!(skipped.is_empty(), "no skips on a clean synthetic root");

        // Available-space helper on a real directory returns a number.
        #[cfg(unix)]
        assert!(available_bytes(&tmp).is_some());

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn nested_dir_symlinks_are_skipped_and_dangling_links_disclosed() {
        let tmp = tempfile_dir("rm692-symlinks");
        let real = tmp.join("real");
        std::fs::create_dir_all(real.join("deep")).unwrap();
        std::fs::write(real.join("a.jsonl"), "hello").unwrap();
        let root = tmp.join("root");
        std::fs::create_dir_all(&root).unwrap();
        // Directory symlink INTO the walked root — must be skipped, not followed.
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, root.join("linked-real")).unwrap();
        // Dangling symlink — disclosed.
        #[cfg(unix)]
        std::os::unix::fs::symlink(tmp.join("missing-target"), root.join("dangling")).unwrap();

        let mut totals = WalkTotals {
            files: 0,
            logical_bytes: 0,
            sidecars: 0,
            sidecar_bytes: 0,
            largest: Vec::new(),
        };
        let mut skipped = Vec::new();
        walk_dir(&root, &mut totals, &mut skipped);
        assert_eq!(totals.files, 0, "nothing real under root itself");
        #[cfg(unix)]
        assert_eq!(
            skipped.len(),
            2,
            "dir symlink + dangling symlink both disclosed"
        );
        std::fs::remove_dir_all(&tmp).ok();
    }

    fn tempfile_dir(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("agenttrace-rm692-{label}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
