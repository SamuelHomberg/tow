mod cli;
mod color;
mod config;
mod git;
mod model;
mod priority;
mod render;
mod select;
mod size;
mod sort;
mod walk;

use std::collections::HashSet;
use std::io::Write;

use anyhow::Result;
use clap::Parser;
use terminal_size::{Height, Width, terminal_size};

use cli::Cli;

fn main() {
    if let Err(e) = run() {
        if is_broken_pipe(&e) {
            // Downstream closed the pipe (e.g. `tow | head`); exit quietly.
            return;
        }
        eprintln!("tow: {e:#}");
        std::process::exit(1);
    }
}

fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .map(|io| io.kind() == std::io::ErrorKind::BrokenPipe)
            .unwrap_or(false)
    })
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    let rules = config::load(&cli)?;
    if cli.dump_config {
        print!("{}", config::dump(&rules));
        return Ok(());
    }
    let ruleset = priority::RuleSet::compile(&rules)?;

    // Resolve terminal size and the screen-fit budget.
    let (term_w, term_h) = match terminal_size() {
        Some((Width(w), Height(h))) => (Some(w), Some(h)),
        None => (None, None),
    };
    let tty = term_w.is_some();

    let width = cli
        .width
        .or_else(|| (rules.defaults.width > 0).then_some(rules.defaults.width))
        .or_else(|| term_w.map(|w| w as usize));
    let height = if cli.all_dirs {
        None
    } else if let Some(h) = cli.height {
        Some(h)
    } else if rules.defaults.height > 0 {
        Some(rules.defaults.height)
    } else if rules.display.collapse && tty {
        term_h.map(|h| h as usize)
    } else {
        None
    };
    // Leave a little room for the trailing report line.
    let height_budget = height.map(|h| {
        if cli.noreport {
            h
        } else {
            h.saturating_sub(2)
        }
    });

    let hide_gitignored = if cli.gitignore {
        true
    } else if cli.no_gitignore {
        false
    } else {
        rules.display.gitignore
    };

    let cfg = cli::Config::from_cli(&cli, height_budget, width, hide_gitignored, &rules.defaults)?;
    let painter = color::Painter::new(cfg.color);

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let multiple = cli.paths.len() > 1;

    for (i, path) in cli.paths.iter().enumerate() {
        if multiple && i > 0 {
            writeln!(out)?;
        }

        let root_display = path.to_string_lossy().into_owned();
        let walk_output = walk::build_tree(path, &root_display, &cfg)?;
        if let Some(cap) = walk_output.truncated {
            let (flag, n) = match cap {
                walk::Cap::Files => ("--max-files", cfg.max_files),
                walk::Cap::Dirs => ("--max-dirs", cfg.max_dirs),
            };
            eprintln!(
                "tow: hit the {flag} limit ({n}); the tree is incomplete — use {flag} 0 to see all"
            );
        }
        let mut tree = walk_output.tree;

        // Resolve git history before pruning so every file has its metadata.
        if (cfg.show_commits || cfg.use_commit_times)
            && let Some(repo) = git::discover(path)
        {
            let mut paths = HashSet::new();
            model::collect_file_paths(&tree, &mut paths);
            if !paths.is_empty() {
                let commits = git::last_commits(&repo, &paths, cfg.max_commits)?;
                model::annotate(&mut tree, &commits);
            }
        }

        // Directory sizes must be computed over the full tree, before files
        // are collapsed away.
        if cfg.size_mode != cli::SizeMode::None {
            size::accumulate(&mut tree);
        }

        select::prune(&mut tree, &cfg, &ruleset);
        if cfg.prune {
            select::prune_empty(&mut tree);
        }
        model::compute_metrics(&mut tree);

        if let Some(n) = cfg.recent {
            render::render_recent(&tree, n, &cfg, &painter, &mut out)?;
        } else {
            render::render_tree(&tree, &cfg, &painter, &ruleset, &mut out)?;
            if !cfg.noreport {
                render::render_report(&tree, &mut out)?;
            }
        }
    }

    Ok(())
}
