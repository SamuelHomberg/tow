use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use globset::{Glob, GlobSet, GlobSetBuilder};

/// tow — a quick overview tree for unknown or forgotten projects.
#[derive(Parser, Debug)]
#[command(
    name = "tow",
    version,
    about = "A quick overview tree for unknown or forgotten projects",
    long_about = None,
    disable_help_flag = true
)]
pub struct Cli {
    /// Print help
    #[arg(long = "help", action = clap::ArgAction::Help)]
    pub help: Option<bool>,

    /// Directories to list (defaults to the current directory)
    #[arg(value_name = "PATH", default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Include hidden files (those beginning with a dot)
    #[arg(short = 'a', long = "all")]
    pub all: bool,

    /// Respect .gitignore and hide ignored files (shown by default)
    #[arg(long = "gitignore")]
    pub gitignore: bool,

    /// Maximum display depth of the directory tree
    #[arg(short = 'L', long = "max-depth", value_name = "N")]
    pub max_depth: Option<usize>,

    /// List directories only
    #[arg(short = 'd', long = "dirs-only")]
    pub dirs_only: bool,

    /// Print the full path prefix for each file
    #[arg(short = 'f', long = "full-path")]
    pub full_path: bool,

    /// Files shown per file-type per directory (default 2)
    #[arg(long = "limit", value_name = "N", default_value_t = 2)]
    pub limit: usize,

    /// How to choose the "exemplary" files when a directory is collapsed
    #[arg(long = "select", value_enum, default_value_t = Select::Important)]
    pub select: Select,

    /// Order files by: name, mtime, commits, size, version, or none
    #[arg(long = "sort", value_enum)]
    pub sort: Option<Sort>,

    /// Reverse the sort order
    #[arg(short = 'r', long = "reverse")]
    pub reverse: bool,

    /// List files before directories
    #[arg(long = "filesfirst")]
    pub filesfirst: bool,

    /// Annotate files with their last commit (on by default inside a repo)
    #[arg(long = "commits")]
    pub commits: bool,

    /// Do not annotate files with their last commit
    #[arg(long = "no-commits")]
    pub no_commits: bool,

    /// Flat list of the N most recently committed files
    #[arg(long = "recent", value_name = "N", num_args = 0..=1, default_missing_value = "10", require_equals = true)]
    pub recent: Option<usize>,

    /// Print the size of each file in bytes
    #[arg(short = 's', long = "size")]
    pub size: bool,

    /// Print sizes in a human readable format (powers of 1024)
    #[arg(short = 'h', long = "human")]
    pub human: bool,

    /// Like -h but use SI units (powers of 1000)
    #[arg(long = "si")]
    pub si: bool,

    /// Report directory sizes as the accumulation of their contents
    #[arg(long = "du")]
    pub du: bool,

    /// Print the last modification date of each file
    #[arg(short = 'D', long = "date")]
    pub date: bool,

    /// Format dates with strftime syntax (implies -D)
    #[arg(long = "timefmt", value_name = "FORMAT")]
    pub timefmt: Option<String>,

    /// List only files matching the glob pattern (repeatable)
    #[arg(short = 'P', value_name = "PATTERN")]
    pub include: Vec<String>,

    /// Do not list files matching the glob pattern (repeatable)
    #[arg(short = 'I', value_name = "PATTERN")]
    pub exclude: Vec<String>,

    /// Prune empty directories from the output
    #[arg(long = "prune")]
    pub prune: bool,

    /// Omit the final file/directory report
    #[arg(long = "noreport")]
    pub noreport: bool,

    /// Disable the "important files" boost
    #[arg(long = "no-important")]
    pub no_important: bool,

    /// Append a '/' to directories, '*' to executables, '@' to symlinks
    #[arg(short = 'F', long = "classify")]
    pub classify: bool,

    /// Do not print indentation lines
    #[arg(short = 'i', long = "noindent")]
    pub noindent: bool,

    /// Follow symbolic links to directories
    #[arg(short = 'l', long = "follow")]
    pub follow_links: bool,

    /// Force color output even when not a tty
    #[arg(short = 'C', long)]
    pub color_always: bool,

    /// Disable color output
    #[arg(short = 'n', long)]
    pub color_never: bool,

    /// When to use color: auto, always, never
    #[arg(long = "color", value_name = "WHEN")]
    pub color: Option<String>,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Select {
    /// Important files, then most-recently-committed, then mtime (default)
    Important,
    /// Most recently changed by commit (falls back to mtime)
    Recent,
    /// Most recently modified (mtime)
    Modified,
    /// Alphabetical
    Name,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Name,
    Mtime,
    Commits,
    Size,
    Version,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Mtime,
    Commits,
    Size,
    Version,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeMode {
    None,
    Bytes,
    Human,
    Si,
}

/// Resolved, normalized configuration passed to the rest of the program.
#[derive(Debug)]
pub struct Config {
    pub color: ColorMode,
    pub all: bool,
    pub gitignore: bool,
    pub max_depth: Option<usize>,
    pub dirs_only: bool,
    pub full_path: bool,
    pub limit: usize,
    pub important_boost: bool,
    pub sort_key: SortKey,
    pub reverse: bool,
    pub files_first: bool,
    pub show_commits: bool,
    pub use_commit_times: bool,
    pub recent: Option<usize>,
    pub size_mode: SizeMode,
    pub du: bool,
    pub show_date: bool,
    pub timefmt: String,
    pub prune: bool,
    pub noreport: bool,
    pub noindent: bool,
    pub classify: bool,
    pub follow_links: bool,
    pub include: Option<GlobSet>,
    pub exclude: Option<GlobSet>,
}

impl Config {
    pub fn from_cli(cli: &Cli) -> anyhow::Result<Config> {
        let color = resolve_color(cli);
        let sort_key = match cli.sort {
            Some(Sort::Name) => SortKey::Name,
            Some(Sort::Mtime) => SortKey::Mtime,
            Some(Sort::Commits) => SortKey::Commits,
            Some(Sort::Size) => SortKey::Size,
            Some(Sort::Version) => SortKey::Version,
            Some(Sort::None) => SortKey::None,
            None => match cli.select {
                Select::Name => SortKey::Name,
                _ => SortKey::Commits,
            },
        };

        let important_boost = !cli.no_important && cli.select == Select::Important;

        let use_commit_times =
            cli.select == Select::Important || cli.select == Select::Recent || sort_key == SortKey::Commits;
        let show_commits = !cli.no_commits || cli.commits || cli.recent.is_some();

        let size_mode = if cli.si {
            SizeMode::Si
        } else if cli.human {
            SizeMode::Human
        } else if cli.size || cli.du {
            SizeMode::Bytes
        } else {
            SizeMode::None
        };

        let timefmt = cli
            .timefmt
            .clone()
            .unwrap_or_else(|| "%Y-%m-%d %H:%M".to_string());

        Ok(Config {
            color,
            all: cli.all,
            gitignore: cli.gitignore,
            max_depth: cli.max_depth,
            dirs_only: cli.dirs_only,
            full_path: cli.full_path,
            limit: cli.limit,
            important_boost,
            sort_key,
            reverse: cli.reverse,
            files_first: cli.filesfirst,
            show_commits,
            use_commit_times,
            recent: cli.recent,
            size_mode,
            du: cli.du,
            show_date: cli.date || cli.timefmt.is_some(),
            timefmt,
            prune: cli.prune,
            noreport: cli.noreport,
            noindent: cli.noindent,
            classify: cli.classify,
            follow_links: cli.follow_links,
            include: build_globs(&cli.include)?,
            exclude: build_globs(&cli.exclude)?,
        })
    }
}

fn resolve_color(cli: &Cli) -> ColorMode {
    if cli.color_never {
        return ColorMode::Never;
    }
    if cli.color_always {
        return ColorMode::Always;
    }
    if std::env::var_os("CLICOLOR_FORCE").is_some() {
        return ColorMode::Always;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return ColorMode::Never;
    }
    match cli.color.as_deref() {
        Some("always") => ColorMode::Always,
        Some("never") => ColorMode::Never,
        _ => ColorMode::Auto,
    }
}

fn build_globs(patterns: &[String]) -> anyhow::Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        // tree treats '|' as a separator between alternate patterns.
        for alt in pat.split('|') {
            let alt = alt.trim();
            if alt.is_empty() {
                continue;
            }
            let glob = Glob::new(alt)?;
            builder.add(glob);
        }
    }
    Ok(Some(builder.build()?))
}
