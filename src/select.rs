use std::collections::BTreeMap;

use crate::cli::Config;
use crate::model::{DirNode, Node, TypeCount, type_label};
use crate::priority::RuleSet;
use crate::sort::{nat_cmp, sort_nodes};

/// Collapse crowded directories: keep entrypoints + anchors and a few examples
/// of each file type, and summarize the rest.
pub fn prune(root: &mut DirNode, cfg: &Config, rules: &RuleSet) {
    prune_dir(root, cfg, rules);
}

fn prune_dir(dir: &mut DirNode, cfg: &Config, rules: &RuleSet) {
    let children = std::mem::take(&mut dir.children);
    let (mut dirs, mut files): (Vec<Node>, Vec<Node>) =
        children.into_iter().partition(|n| n.is_dir());

    // Compute priority tiers for each file.
    for f in &mut files {
        if let Node::File(fnode) = f {
            fnode.tier = rules.file_tier(&fnode.name);
        }
    }

    // Entrypoints (tier 0) are always shown; anchors (tier 1) only under the
    // "important" boost (default), disabled by --no-important / --select.
    let (mut primary, rest1): (Vec<Node>, Vec<Node>) =
        files.into_iter().partition(|f| matches!(f, Node::File(fnode) if fnode.tier == 0));
    let (mut anchors, rest): (Vec<Node>, Vec<Node>) = if cfg.important_boost {
        rest1.into_iter().partition(|f| matches!(f, Node::File(fnode) if fnode.tier == 1))
    } else {
        (Vec::new(), rest1)
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

    // Entrypoints first, then anchors (both name-sorted), then capped groups.
    primary.sort_by(|a, b| nat_cmp(a.name(), b.name()));
    anchors.sort_by(|a, b| nat_cmp(a.name(), b.name()));
    if cfg.reverse {
        primary.reverse();
        anchors.reverse();
    }
    let mut files_final = primary;
    files_final.extend(anchors);
    files_final.extend(shown);

    // Directories: class order (protected -> normal -> noise), then name.
    dirs.sort_by(|a, b| {
        rules
            .dir_class(a.name())
            .cmp(&rules.dir_class(b.name()))
            .then_with(|| nat_cmp(a.name(), b.name()))
    });
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
            prune_dir(d, cfg, rules);
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
