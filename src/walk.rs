use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::Result;
use ignore::WalkBuilder;

use crate::cli::Config;
use crate::model::{DirNode, FileNode, Node};

struct RawEntry {
    name: String,
    rel: PathBuf,
    path: PathBuf,
    size: u64,
    mtime: i64,
    is_dir: bool,
    is_exec: bool,
    is_symlink: bool,
}

/// Walk `root` (respecting depth, hidden files, -P/-I patterns, and
/// optionally .gitignore) and build an in-memory tree.
pub fn build_tree(root: &Path, root_display: &str, cfg: &Config) -> Result<DirNode> {
    let root_abs = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf());

    let mut builder = WalkBuilder::new(&root_abs);
    builder
        .hidden(!cfg.all)
        .git_ignore(cfg.gitignore)
        .git_global(cfg.gitignore)
        .git_exclude(cfg.gitignore)
        .require_git(false)
        .parents(true)
        .follow_links(cfg.follow_links);
    if let Some(d) = cfg.max_depth {
        builder.max_depth(Some(d));
    }

    let exclude = cfg.exclude.clone();
    builder.filter_entry(move |entry| {
        if let Some(set) = &exclude
            && let Some(name) = entry.file_name().to_str()
            && set.is_match(name)
        {
            return false;
        }
        true
    });

    let mut children_of: HashMap<PathBuf, Vec<RawEntry>> = HashMap::new();
    for result in builder.build() {
        let entry = result?;
        let path = entry.path().to_path_buf();
        if path == root_abs {
            continue;
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        let ft_is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let is_symlink = entry.path_is_symlink();
        let is_dir = ft_is_dir && !is_symlink;

        if cfg.dirs_only && !is_dir {
            continue;
        }

        // -P: include only matching files (directories are always kept so we
        // can descend into them).
        if !is_dir
            && let Some(set) = &cfg.include
            && !set.is_match(&name)
        {
            continue;
        }

        let meta = entry.metadata().ok();
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let mtime = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(to_unix)
            .unwrap_or(0);
        let is_exec = meta.as_ref().map(is_exec).unwrap_or(false);

        let rel = path
            .strip_prefix(&root_abs)
            .map(Path::to_path_buf)
            .unwrap_or_default();

        let parent = path.parent().unwrap_or(&root_abs).to_path_buf();
        children_of.entry(parent).or_default().push(RawEntry {
            name,
            rel,
            path,
            size,
            mtime,
            is_dir,
            is_exec,
            is_symlink,
        });
    }

    let root_meta = std::fs::metadata(&root_abs).ok();
    let root_mtime = root_meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(to_unix)
        .unwrap_or(0);

    Ok(build_dir(
        &root_abs,
        &PathBuf::new(),
        root_display.to_string(),
        root_mtime,
        &mut children_of,
    ))
}

fn build_dir(
    path: &Path,
    rel: &Path,
    name: String,
    mtime: i64,
    children_of: &mut HashMap<PathBuf, Vec<RawEntry>>,
) -> DirNode {
    let raw_children = children_of.remove(path).unwrap_or_default();
    let mut children = Vec::with_capacity(raw_children.len());

    for raw in raw_children {
        if raw.is_dir {
            let sub = build_dir(&raw.path, &raw.rel, raw.name, raw.mtime, children_of);
            children.push(Node::Dir(sub));
        } else {
            children.push(Node::File(FileNode {
                name: raw.name,
                rel: raw.rel,
                path: raw.path,
                size: raw.size,
                mtime: raw.mtime,
                is_exec: raw.is_exec,
                is_symlink: raw.is_symlink,
                important: false,
                git: None,
            }));
        }
    }

    DirNode {
        name,
        rel: rel.to_path_buf(),
        size: 0,
        mtime,
        children,
        hidden: Vec::new(),
    }
}

fn to_unix(t: std::time::SystemTime) -> Option<i64> {
    t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs() as i64)
}

#[cfg(unix)]
fn is_exec(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_exec(_: &std::fs::Metadata) -> bool {
    false
}
