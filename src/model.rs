use std::path::PathBuf;

/// Git metadata for the most recent commit that touched a file.
#[derive(Debug, Clone)]
pub struct GitInfo {
    /// Short commit hash (first 7 hex chars).
    pub hash: String,
    /// Commit timestamp (Unix seconds).
    pub timestamp: i64,
    /// Trimmed first line of the commit message.
    pub subject: String,
}

/// A collapsed count of files grouped by file type (extension).
#[derive(Debug, Clone)]
pub struct TypeCount {
    /// Display label, e.g. `.py` or `(no ext)`.
    pub label: String,
    pub count: usize,
}

#[derive(Debug)]
pub struct FileNode {
    pub name: String,
    /// Path relative to the walk root (for `-f` output).
    pub rel: PathBuf,
    /// Absolute path (for git lookups).
    pub path: PathBuf,
    pub size: u64,
    pub mtime: i64,
    pub is_exec: bool,
    pub is_symlink: bool,
    /// Priority tier: 0 = entrypoint, 1 = anchor, 2 = ordinary.
    pub tier: u8,
    pub git: Option<GitInfo>,
}

#[derive(Debug)]
pub struct DirNode {
    pub name: String,
    pub rel: PathBuf,
    pub size: u64,
    pub mtime: i64,
    pub children: Vec<Node>,
    /// Files not shown due to the per-type cap, grouped by extension.
    pub hidden: Vec<TypeCount>,
    /// Precomputed rendered line count of this subtree (for screen collapse).
    pub height: usize,
    /// Precomputed number of directories in this subtree (including itself).
    pub dir_count: usize,
    /// Precomputed deepest directory nesting in this subtree (1 = leaf dir).
    pub max_depth: usize,
    /// Precomputed widest fan-out (most subdirectories of any one directory).
    pub max_fanout: usize,
}

#[derive(Debug)]
pub enum Node {
    Dir(DirNode),
    File(FileNode),
}

impl Node {
    pub fn name(&self) -> &str {
        match self {
            Node::Dir(d) => &d.name,
            Node::File(f) => &f.name,
        }
    }

    pub fn rel(&self) -> &std::path::Path {
        match self {
            Node::Dir(d) => &d.rel,
            Node::File(f) => &f.rel,
        }
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, Node::Dir(_))
    }

    pub fn size(&self) -> u64 {
        match self {
            Node::Dir(d) => d.size,
            Node::File(f) => f.size,
        }
    }

    pub fn mtime(&self) -> i64 {
        match self {
            Node::Dir(d) => d.mtime,
            Node::File(f) => f.mtime,
        }
    }

    /// Last-commit time if available, otherwise the filesystem mtime.
    pub fn change_time(&self) -> i64 {
        match self {
            Node::File(f) => f.git.as_ref().map(|g| g.timestamp).unwrap_or(f.mtime),
            Node::Dir(d) => d.mtime,
        }
    }

    pub fn is_exec(&self) -> bool {
        matches!(self, Node::File(f) if f.is_exec)
    }

    pub fn is_symlink(&self) -> bool {
        matches!(self, Node::File(f) if f.is_symlink)
    }

    pub fn git(&self) -> Option<&GitInfo> {
        match self {
            Node::File(f) => f.git.as_ref(),
            Node::Dir(_) => None,
        }
    }

    /// Rendered line count of this node's subtree (precomputed).
    pub fn subtree_height(&self) -> usize {
        match self {
            Node::Dir(d) => d.height,
            Node::File(_) => 1,
        }
    }

    /// Number of directories in this node's subtree (precomputed).
    pub fn subtree_dirs(&self) -> usize {
        match self {
            Node::Dir(d) => d.dir_count,
            Node::File(_) => 0,
        }
    }

    /// Deepest directory nesting in this node's subtree (precomputed).
    pub fn subtree_depth(&self) -> usize {
        match self {
            Node::Dir(d) => d.max_depth,
            Node::File(_) => 0,
        }
    }

    /// Widest fan-out in this node's subtree (precomputed).
    pub fn subtree_fanout(&self) -> usize {
        match self {
            Node::Dir(d) => d.max_fanout,
            Node::File(_) => 0,
        }
    }
}

/// Return the lowercase extension of a file name, or `None` if there is none.
/// Treats leading-dot names (`.gitignore`) as having no extension.
pub fn extension_of(name: &str) -> Option<String> {
    let base = name.rsplit('/').next().unwrap_or(name);
    let dot = base.rfind('.')?;
    if dot == 0 {
        return None;
    }
    let ext = &base[dot + 1..];
    if ext.is_empty() {
        return None;
    }
    Some(ext.to_lowercase())
}

/// Human label for a file's type, used in the "N more" summary.
pub fn type_label(name: &str) -> String {
    match extension_of(name) {
        Some(e) => format!(".{e}"),
        None => "(no ext)".to_string(),
    }
}

/// Collect the absolute paths of every file in the tree.
pub fn collect_file_paths(root: &DirNode, out: &mut std::collections::HashSet<PathBuf>) {
    for c in &root.children {
        match c {
            Node::File(f) => {
                out.insert(f.path.clone());
            }
            Node::Dir(d) => collect_file_paths(d, out),
        }
    }
}

/// Annotate each file with its git metadata (if present in `map`).
pub fn annotate(root: &mut DirNode, map: &std::collections::HashMap<PathBuf, GitInfo>) {
    for c in &mut root.children {
        match c {
            Node::File(f) => {
                f.git = map.get(&f.path).cloned();
            }
            Node::Dir(d) => annotate(d, map),
        }
    }
}

/// Precompute each directory's rendered height, directory count, nesting depth,
/// and max fan-out in a single bottom-up pass, so the screen-fit collapse runs
/// in linear time.
pub fn compute_metrics(root: &mut DirNode) -> (usize, usize, usize, usize) {
    let mut height = 1usize;
    let mut dir_count = 1usize;
    let mut depth = 1usize;
    let mut fanout = 0usize;
    let mut max_fanout = 0usize;
    for child in &mut root.children {
        match child {
            Node::File(_) => height += 1,
            Node::Dir(d) => {
                let (h, c, dp, f) = compute_metrics(d);
                height += h;
                dir_count += c;
                depth = depth.max(dp + 1);
                fanout += 1;
                max_fanout = max_fanout.max(f);
            }
        }
    }
    if !root.hidden.is_empty() {
        height += 1;
    }
    max_fanout = max_fanout.max(fanout);
    root.height = height;
    root.dir_count = dir_count;
    root.max_depth = depth;
    root.max_fanout = max_fanout;
    (height, dir_count, depth, max_fanout)
}
