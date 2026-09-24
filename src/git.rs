use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use git2::{Commit, Repository};

use crate::model::GitInfo;

/// Attempt to discover the git repository containing `path`.
pub fn discover(path: &Path) -> Option<Repository> {
    Repository::discover(path).ok()
}

/// Compute the most recent commit that touched each of the requested
/// (absolute) paths. Returns a map keyed by the original absolute path.
///
/// Walks history newest-first and stops as soon as every requested path has
/// been matched, so it stays cheap for the paths tow actually cares about.
pub fn last_commits(
    repo: &Repository,
    paths: &HashSet<PathBuf>,
    max_commits: usize,
) -> Result<HashMap<PathBuf, GitInfo>> {
    // A freshly `git init`ed repository has no commits (unborn HEAD); there is
    // nothing to annotate.
    if repo.head().is_err() {
        return Ok(HashMap::new());
    }

    let workdir = repo.workdir().unwrap_or_else(|| Path::new("."));
    let mut remaining: HashSet<PathBuf> = paths
        .iter()
        .filter_map(|p| p.strip_prefix(workdir).ok().map(Path::to_path_buf))
        .collect();

    let mut result = HashMap::new();
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let mut scanned = 0usize;
    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;
        let parent_tree = commit.parents().next().and_then(|p| p.tree().ok());
        let diff = match &parent_tree {
            Some(pt) => repo.diff_tree_to_tree(Some(pt), Some(&tree), None)?,
            None => repo.diff_tree_to_tree(None, Some(&tree), None)?,
        };

        let hash = short_hash(&commit);
        let timestamp = commit.time().seconds();
        let subject = subject(&commit);

        for delta in diff.deltas() {
            if let Some(path) = delta.new_file().path() {
                let path = path.to_path_buf();
                if remaining.remove(&path) {
                    result.insert(
                        workdir.join(&path),
                        GitInfo {
                            hash: hash.clone(),
                            timestamp,
                            subject: subject.clone(),
                        },
                    );
                }
            }
        }

        if remaining.is_empty() {
            break;
        }
        scanned += 1;
        if max_commits > 0 && scanned >= max_commits {
            break;
        }
    }

    Ok(result)
}

fn short_hash(commit: &Commit) -> String {
    let full = commit.id().to_string();
    full.chars().take(7).collect()
}

fn subject(commit: &Commit) -> String {
    let s = commit.summary().unwrap_or("").trim();
    let count = s.chars().count();
    let mut out: String = s.chars().take(48).collect();
    if count > 48 {
        out.push('…');
    }
    out
}
