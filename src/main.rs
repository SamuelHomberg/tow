mod cli;
mod color;
mod git;
mod important;
mod model;
mod render;
mod select;
mod size;
mod sort;
mod walk;

use std::collections::HashSet;
use std::io::Write;

use anyhow::Result;
use clap::Parser;

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
    let cfg = cli::Config::from_cli(&cli)?;
    let painter = color::Painter::new(cfg.color);

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let multiple = cli.paths.len() > 1;

    for (i, path) in cli.paths.iter().enumerate() {
        if multiple && i > 0 {
            writeln!(out)?;
        }

        let root_display = path.to_string_lossy().into_owned();
        let mut tree = walk::build_tree(path, &root_display, &cfg)?;

        // Resolve git history (needed for commit annotations or commit-time
        // ordering) before pruning so every file has its metadata.
        if (cfg.show_commits || cfg.use_commit_times)
            && let Some(repo) = git::discover(path)
        {
            let mut paths = HashSet::new();
            model::collect_file_paths(&tree, &mut paths);
            if !paths.is_empty() {
                let commits = git::last_commits(&repo, &paths)?;
                model::annotate(&mut tree, &commits);
            }
        }

        // Directory sizes must be computed over the full tree, before files
        // are collapsed away.
        if cfg.du {
            size::accumulate(&mut tree);
        }

        select::prune(&mut tree, &cfg);
        if cfg.prune {
            select::prune_empty(&mut tree);
        }

        if let Some(n) = cfg.recent {
            render::render_recent(&tree, n, &cfg, &painter, &mut out)?;
        } else {
            render::render_tree(&tree, &cfg, &painter, &mut out)?;
            if !cfg.noreport {
                render::render_report(&tree, &mut out)?;
            }
        }
    }

    Ok(())
}
