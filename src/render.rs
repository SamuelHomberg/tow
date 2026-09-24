use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use jiff::Timestamp;

use crate::cli::{Config, SizeMode};
use crate::color::Painter;
use crate::model::{DirNode, Node, TypeCount};
use crate::priority::RuleSet;
use crate::size::human;

/// Does `n` lines fit within the remaining budget?
fn fits(remaining: Option<usize>, n: usize) -> bool {
    match remaining {
        None => true,
        Some(r) => r >= n,
    }
}

fn consume(remaining: Option<usize>, n: usize) -> Option<usize> {
    remaining.map(|r| r.saturating_sub(n))
}

/// Render the full tree for a single root.
pub fn render_tree(
    root: &DirNode,
    cfg: &Config,
    painter: &Painter,
    rules: &RuleSet,
    out: &mut impl Write,
) -> Result<()> {
    writeln!(out, "{}", painter.dir(&root.name))?;
    render_entries(
        &root.children,
        &root.hidden,
        &root.name,
        "",
        cfg,
        painter,
        rules,
        cfg.height,
        out,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_entries(
    children: &[Node],
    hidden: &[TypeCount],
    root_label: &str,
    prefix: &str,
    cfg: &Config,
    painter: &Painter,
    rules: &RuleSet,
    remaining: Option<usize>,
    out: &mut impl Write,
) -> Result<()> {
    let n = children.len();

    // Decide which children render vs hide, simulating budget consumption in
    // order. A directory only renders when its whole subtree fits (unless it is
    // protected); otherwise it is hidden and counted.
    let mut sim = remaining;
    let mut render_mask = vec![false; n];
    let mut hidden_dirs = 0usize;
    let mut hidden_roots = 0usize;
    let mut hidden_depth = 0usize;
    let mut hidden_width = 0usize;
    for (i, child) in children.iter().enumerate() {
        match child {
            Node::Dir(_) => {
                let protected = rules.is_protected_dir(child.name());
                let h = child.subtree_height();
                if protected || fits(sim, h) {
                    render_mask[i] = true;
                    sim = consume(sim, h);
                } else {
                    hidden_dirs += child.subtree_dirs();
                    hidden_roots += 1;
                    hidden_depth = hidden_depth.max(child.subtree_depth());
                    hidden_width = hidden_width.max(child.subtree_fanout());
                }
            }
            Node::File(_) => {
                if fits(sim, 1) {
                    render_mask[i] = true;
                    sim = consume(sim, 1);
                }
            }
        }
    }

    let has_hidden_files = !hidden.is_empty();
    let has_hidden_dirs = hidden_dirs > 0;
    let trailing = usize::from(has_hidden_files) + usize::from(has_hidden_dirs);
    let last_rendered = (0..n).rev().find(|&i| render_mask[i]);

    for (i, child) in children.iter().enumerate() {
        if !render_mask[i] {
            continue;
        }
        let is_last_line = Some(i) == last_rendered && trailing == 0;
        let connector = if cfg.noindent {
            ""
        } else if is_last_line {
            "└── "
        } else {
            "├── "
        };

        let line = format_line(child, root_label, prefix, connector, cfg, painter);
        writeln!(out, "{line}")?;

        if let Node::Dir(d) = child {
            let child_prefix = if cfg.noindent {
                String::new()
            } else if is_last_line {
                format!("{prefix}    ")
            } else {
                format!("{prefix}│   ")
            };
            // The subtree's full height was already accounted for, so render it
            // in its entirety.
            render_entries(
                &d.children,
                &d.hidden,
                root_label,
                &child_prefix,
                cfg,
                painter,
                rules,
                None,
                out,
            )?;
        }
    }

    if has_hidden_files {
        render_hidden(hidden, prefix, cfg, painter, !has_hidden_dirs, out)?;
    }
    if has_hidden_dirs {
        let summary = HiddenDirs {
            count: hidden_dirs,
            roots: hidden_roots,
            depth: hidden_depth,
            width: hidden_width,
        };
        render_hidden_dirs(&summary, prefix, cfg, painter, out)?;
    }
    Ok(())
}

/// Build a single rendered line (prefix + connector + content).
fn format_line(
    node: &Node,
    root_label: &str,
    prefix: &str,
    connector: &str,
    cfg: &Config,
    painter: &Painter,
) -> String {
    let size = size_str(node, cfg);
    let cls = if cfg.classify { classifier(node) } else { "" };
    let plain = plain_name(node, root_label, cfg);
    let date = if cfg.show_date {
        Some(format_time(node.mtime(), &cfg.timefmt))
    } else {
        None
    };

    // Width of everything before the commit annotation (plain, no ANSI).
    let mut used = prefix.chars().count() + connector.chars().count();
    if size.is_some() {
        used += 9; // "{:>8} "
    }
    used += plain.chars().count();
    used += cls.chars().count();
    if let Some(d) = &date {
        used += 2 + d.chars().count();
    }

    let mut line = String::new();
    line.push_str(prefix);
    line.push_str(connector);
    if let Some(s) = &size {
        line.push_str(&format!("{s:>8} "));
    }
    line.push_str(&color_name(node, root_label, cfg, painter));
    line.push_str(cls);
    if let Some(d) = &date {
        line.push_str(&format!("  {d}"));
    }
    if cfg.show_commits
        && let Some(g) = node.git()
    {
        let date_str = format_time(g.timestamp, &cfg.timefmt);
        let meta = commit_meta(&g.hash, &date_str, &g.subject, used, cfg);
        line.push_str(&painter.meta(&meta));
    }
    line
}

fn plain_name(node: &Node, root_label: &str, cfg: &Config) -> String {
    if !cfg.full_path {
        node.name().to_string()
    } else {
        full_path_string(node, root_label)
    }
}

fn color_name(node: &Node, root_label: &str, cfg: &Config, painter: &Painter) -> String {
    if !cfg.full_path {
        painter.name(node)
    } else {
        let full = full_path_string(node, root_label);
        painter.path(&full, node)
    }
}

fn full_path_string(node: &Node, root_label: &str) -> String {
    let rel = node.rel();
    if rel.as_os_str().is_empty() {
        root_label.to_string()
    } else {
        format!("{}/{}", root_label.trim_end_matches('/'), rel.display())
    }
}

fn size_str(node: &Node, cfg: &Config) -> Option<String> {
    if cfg.size_mode == SizeMode::None {
        return None;
    }
    let bytes = node.size();
    Some(match cfg.size_mode {
        SizeMode::None => unreachable!(),
        SizeMode::Bytes => bytes.to_string(),
        SizeMode::Human => human(bytes, false),
        SizeMode::Si => human(bytes, true),
    })
}

fn classifier(node: &Node) -> &'static str {
    if node.is_dir() {
        "/"
    } else if node.is_symlink() {
        "@"
    } else if node.is_exec() {
        "*"
    } else {
        ""
    }
}

/// Build the commit annotation, truncating it to fit the available width.
/// Preference order when tight: full -> drop hash -> drop date -> truncate
/// subject -> nothing.
fn commit_meta(hash: &str, date: &str, subject: &str, used: usize, cfg: &Config) -> String {
    let full = format!("  {hash}  {date}  {subject}");
    if cfg.full_commits {
        return full;
    }
    let Some(width) = cfg.width else {
        return full;
    };
    let avail = width.saturating_sub(used);

    if full.chars().count() <= avail {
        return full;
    }
    let no_hash = format!("  {date}  {subject}");
    if no_hash.chars().count() <= avail {
        return no_hash;
    }
    let subject_only = format!("  {subject}");
    if subject_only.chars().count() <= avail {
        return subject_only;
    }
    if avail >= 3 {
        let subj = truncate(subject, avail - 2);
        return format!("  {subj}");
    }
    String::new()
}

/// Truncate `s` to at most `max` chars, appending `…` when truncated.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    if max == 0 {
        return "…".to_string();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

fn render_hidden(
    hidden: &[TypeCount],
    prefix: &str,
    cfg: &Config,
    painter: &Painter,
    last: bool,
    out: &mut impl Write,
) -> Result<()> {
    let total: usize = hidden.iter().map(|t| t.count).sum();
    let text = if hidden.len() == 1 {
        let t = &hidden[0];
        format!("… {total} more {}", t.label)
    } else {
        let parts: Vec<String> = hidden
            .iter()
            .map(|t| format!("{} {}", t.count, t.label))
            .collect();
        format!("… {total} more ({})", parts.join(", "))
    };
    let connector = if cfg.noindent {
        ""
    } else if last {
        "└── "
    } else {
        "├── "
    };
    writeln!(out, "{prefix}{connector}{}", painter.muted(&text))?;
    Ok(())
}

/// Aggregate shape information about directories hidden by the screen budget.
struct HiddenDirs {
    count: usize,
    roots: usize,
    depth: usize,
    width: usize,
}

fn render_hidden_dirs(
    s: &HiddenDirs,
    prefix: &str,
    cfg: &Config,
    painter: &Painter,
    out: &mut impl Write,
) -> Result<()> {
    let noun = if s.count == 1 {
        "directory"
    } else {
        "directories"
    };
    let text = format!("… {} {noun} hidden{} (--all-dirs)", s.count, shape_text(s));
    let connector = if cfg.noindent { "" } else { "└── " };
    writeln!(out, "{prefix}{connector}{}", painter.muted(&text))?;
    Ok(())
}

/// A short structural description of the hidden forest.
fn shape_text(s: &HiddenDirs) -> String {
    if s.count <= 1 {
        return String::new();
    }
    if s.roots == 1 {
        if s.width <= 1 {
            format!(" (a chain {} deep)", s.count)
        } else if s.depth <= 2 {
            format!(" (one directory, {} subdirs)", s.count - 1)
        } else {
            format!(" (≤{} subdirs, {} deep)", s.width, s.depth)
        }
    } else {
        format!(
            " ({} roots, ≤{} subdirs, {} deep)",
            s.roots, s.width, s.depth
        )
    }
}

/// Render the `--recent` flat list: the N most recently committed files.
pub fn render_recent(
    root: &DirNode,
    n: usize,
    cfg: &Config,
    painter: &Painter,
    out: &mut impl Write,
) -> Result<()> {
    let mut items: Vec<(i64, String, String, PathBuf)> = Vec::new();
    collect_recent(root, &mut items);
    items.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.3.cmp(&b.3)));
    items.truncate(n);

    for (ts, hash, subject, rel) in items {
        let date = format_time(ts, &cfg.timefmt);
        writeln!(
            out,
            "{}  {}  {}  {}",
            painter.meta(&hash),
            painter.meta(&date),
            rel.display(),
            subject
        )?;
    }
    Ok(())
}

fn collect_recent(root: &DirNode, out: &mut Vec<(i64, String, String, PathBuf)>) {
    for c in &root.children {
        match c {
            Node::File(f) => {
                if let Some(g) = &f.git {
                    out.push((g.timestamp, g.hash.clone(), g.subject.clone(), f.rel.clone()));
                }
            }
            Node::Dir(d) => collect_recent(d, out),
        }
    }
}

/// Final report line: "N directories, M files".
pub fn render_report(root: &DirNode, out: &mut impl Write) -> Result<()> {
    let (dirs, files) = count(root);
    writeln!(out)?;
    let d = if dirs == 1 { "directory" } else { "directories" };
    let f = if files == 1 { "file" } else { "files" };
    writeln!(out, "{dirs} {d}, {files} {f}")?;
    Ok(())
}

fn count(root: &DirNode) -> (usize, usize) {
    let mut dirs = 0;
    let mut files = 0;
    for c in &root.children {
        match c {
            Node::Dir(d) => {
                dirs += 1;
                let (sd, sf) = count(d);
                dirs += sd;
                files += sf;
            }
            Node::File(_) => files += 1,
        }
    }
    (dirs, files)
}

/// Format a Unix timestamp using the given strftime format, in the system
/// time zone.
fn format_time(secs: i64, fmt: &str) -> String {
    match Timestamp::from_second(secs) {
        Ok(ts) => {
            let zoned = ts.to_zoned(jiff::tz::TimeZone::system());
            zoned.strftime(fmt).to_string()
        }
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn truncates_with_ellipsis() {
        assert_eq!(truncate("hello world", 8), "hello w…");
        assert_eq!(truncate("short", 8), "short");
        assert_eq!(truncate("abc", 0), "…");
    }
}
