use std::collections::BTreeMap;

use crate::cli::Config;
use crate::important;
use crate::model::{DirNode, Node, TypeCount, type_label};
use crate::sort::{nat_cmp, sort_nodes};

/// Collapse crowded directories: keep important files and a few examples of
/// each file type, and summarize the rest.
pub fn prune(root: &mut DirNode, cfg: &Config) {
    prune_dir(root, cfg);
}

fn prune_dir(dir: &mut DirNode, cfg: &Config) {
    let children = std::mem::take(&mut dir.children);
    let (mut dirs, mut files): (Vec<Node>, Vec<Node>) =
        children.into_iter().partition(|n| n.is_dir());

    for f in &mut files {
        if let Node::File(fnode) = f {
            fnode.important = important::is_important(&fnode.name);
        }
    }

    let (mut important, rest): (Vec<Node>, Vec<Node>) = if cfg.important_boost {
        files.into_iter().partition(|f| matches!(f, Node::File(fnode) if fnode.important))
    } else {
        (Vec::new(), files)
    };

    // Group the remaining files by type so we can cap each type and report
    // how many of each type were hidden.
    let mut groups: BTreeMap<String, Vec<Node>> = BTreeMap::new();
    for f in rest {
        groups.entry(type_label(f.name())).or_default().push(f);
    }

    // Order groups by how many files they hold (most common type first).
    let mut groups: Vec<(String, Vec<Node>)> = groups.into_iter().collect();
    groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));

    let mut shown = Vec::new();
    let mut hidden = Vec::new();
    for (label, mut group) in groups {
        sort_nodes(&mut group, cfg);
        let take = cfg.limit.min(group.len());
        let rest_group = group.split_off(take);
        if !rest_group.is_empty() {
            hidden.push(TypeCount {
                label,
                count: rest_group.len(),
            });
        }
        shown.extend(group);
    }

    // Important files first (name-sorted), then the capped type groups.
    important.sort_by(|a, b| nat_cmp(a.name(), b.name()));
    if cfg.reverse {
        important.reverse();
    }
    let mut files_final = important;
    files_final.extend(shown);

    // Directories are always name-sorted (stable structure), honoring reverse.
    dirs.sort_by(|a, b| nat_cmp(a.name(), b.name()));
    if cfg.reverse {
        dirs.reverse();
    }

    let new_children = if cfg.files_first {
        files_final.append(&mut dirs);
        files_final
    } else {
        dirs.append(&mut files_final);
        dirs
    };

    dir.hidden = hidden;
    dir.children = new_children;

    for child in &mut dir.children {
        if let Node::Dir(d) = child {
            prune_dir(d, cfg);
        }
    }
}

/// Remove directories that ended up empty (no shown children, no hidden
/// files). Never removes the root itself.
pub fn prune_empty(dir: &mut DirNode) {
    dir.children.retain_mut(|child| {
        if let Node::Dir(d) = child {
            prune_empty(d);
            !(d.children.is_empty() && d.hidden.is_empty())
        } else {
            true
        }
    });
}
