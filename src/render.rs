use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use jiff::Timestamp;

use crate::cli::{Config, SizeMode};
use crate::color::Painter;
use crate::model::{DirNode, Node, TypeCount};
use crate::size::human;

/// Render the full tree for a single root.
pub fn render_tree(
    root: &DirNode,
    cfg: &Config,
    painter: &Painter,
    out: &mut impl Write,
) -> Result<()> {
    writeln!(out, "{}", painter.dir(&root.name))?;
    render_entries(&root.children, &root.hidden, &root.name, "", cfg, painter, out)
}

fn render_entries(
    children: &[Node],
    hidden: &[TypeCount],
    root_label: &str,
    prefix: &str,
    cfg: &Config,
    painter: &Painter,
    out: &mut impl Write,
) -> Result<()> {
    let n = children.len();
    let has_hidden = !hidden.is_empty();
    for (i, child) in children.iter().enumerate() {
        let last = (i + 1 == n) && !has_hidden;
        let connector = if cfg.noindent {
            ""
        } else if last {
            "└── "
        } else {
            "├── "
        };

        let name = display_name(child, root_label, cfg, painter);
        let mut line = String::new();
        if let Some(s) = size_str(child, cfg) {
            line.push_str(&format!("{s:>8} "));
        }
        line.push_str(&name);
        if cfg.classify {
            line.push_str(classifier(child));
        }
        if cfg.show_date {
            line.push_str(&format!("  {}", format_time(child.mtime(), &cfg.timefmt)));
        }
        if cfg.show_commits
            && let Some(g) = child.git()
        {
            let meta = format!(
                "  {}  {}  {}",
                g.hash,
                format_time(g.timestamp, &cfg.timefmt),
                g.subject
            );
            line.push_str(&painter.meta(&meta));
        }
        writeln!(out, "{prefix}{connector}{line}")?;

        if let Node::Dir(d) = child {
            let child_prefix = if cfg.noindent {
                String::new()
            } else if last {
                format!("{prefix}    ")
            } else {
                format!("{prefix}│   ")
            };
            render_entries(&d.children, &d.hidden, root_label, &child_prefix, cfg, painter, out)?;
        }
    }

    if has_hidden {
        render_hidden(hidden, prefix, cfg, painter, out)?;
    }
    Ok(())
}

fn display_name(node: &Node, root_label: &str, cfg: &Config, painter: &Painter) -> String {
    if !cfg.full_path {
        return painter.name(node);
    }
    let rel = node.rel();
    let full = if rel.as_os_str().is_empty() {
        root_label.to_string()
    } else {
        format!("{}/{}", root_label.trim_end_matches('/'), rel.display())
    };
    painter.path(&full, node)
}

fn size_str(node: &Node, cfg: &Config) -> Option<String> {
    if cfg.size_mode == SizeMode::None {
        return None;
    }
    if node.is_dir() && !cfg.du {
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

fn render_hidden(
    hidden: &[TypeCount],
    prefix: &str,
    cfg: &Config,
    painter: &Painter,
    out: &mut impl Write,
) -> Result<()> {
    if hidden.is_empty() {
        return Ok(());
    }
    let total: usize = hidden.iter().map(|t| t.count).sum();
    let text = if hidden.len() == 1 {
        let t = &hidden[0];
        format!("… {} more {}", total, t.label)
    } else {
        let parts: Vec<String> = hidden
            .iter()
            .map(|t| format!("{} {}", t.count, t.label))
            .collect();
        format!("… {} more ({})", total, parts.join(", "))
    };
    let connector = if cfg.noindent { "" } else { "└── " };
    writeln!(out, "{prefix}{connector}{}", painter.muted(&text))?;
    Ok(())
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
