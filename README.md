# tow

**t**ree **o**verview **w**ithout the overwhelm.

`tow` gives you a quick, readable overview of an unknown or forgotten project.
The classic `tree` prints *everything* — which is useless the moment you hit a
directory with 200 files. `tow` collapses the noise and surfaces what matters.

- Shows gitignored files and directories by default; use `--gitignore` to hide
  them (so `node_modules` and `target/` disappear on request).
- Shows a couple of **exemplary files** per file type, then summarizes the rest
  (`… 7 more .py`) instead of dumping all nine.
- Colors file types consistently, highlighting the extension (`.py` always blue,
  `.rs` always orange, …).
- Optionally annotates each file with its **last commit** (hash, date, subject).
- Optionally reports **directory sizes** (`du`-style).

## Install

```sh
cargo install --path .
```

Or build and run in place:

```sh
cargo build --release
./target/release/tow
```

## Usage

```sh
tow                 # the current directory
tow src tests       # several directories
tow -L 2            # limit depth
tow --du -h         # directory sizes, human readable
tow --recent        # flat list of recently committed files
```

### A quick example

A Python project with a busy `src/`:

```
$ tow
.
├── src
│   ├── main.py
│   ├── utils.py
│   └── … 7 more .py
├── tests
│   ├── test_main.py
│   └── test_utils.py
├── pyproject.toml
└── README.md

2 directories, 6 files
```

Instead of nine `.py` files you see two plus a one-line summary. Files like
`pyproject.toml` and `README.md` are always surfaced even in a crowded root.

### Git history

Inside a git repository, each file is annotated with its last commit by default:

```
$ tow
.
├── src
│   └── lib.rs  91774e2  2026-09-22 08:00  fix: tweak lib something very long that should b…
├── README.md  9bcac00  2026-09-23 07:00  docs: readme
└── main.rs  69adfaf  2026-09-20 10:00  init: add main
```

Use `--no-commits` to hide the annotations. The `--recent` flag prints a flat
list of the most recently changed files — which works even in a freshly cloned
directory, where every file's modification time is just "now":

```
$ tow --recent=3
9bcac00  2026-09-23 07:00  README.md  docs: readme
91774e2  2026-09-22 08:00  src/lib.rs  fix: tweak lib something very long that should b…
69adfaf  2026-09-20 10:00  main.rs  init: add main
```

## How it chooses files

When a directory holds more files than the cap, `tow`:

1. Always shows **important** files (`README*`, `LICENSE*`, `Cargo.toml`,
   `package.json`, `go.mod`, `pyproject.toml`, `Makefile`, `Dockerfile`, …).
2. Groups the rest by file type and shows up to `--limit` (default 2) per type,
   ordered by last-commit time (falling back to modification time, then name).
3. Summarizes everything else per type: `… 7 more .py`, `… 3 more .md`.

This strategy is selectable via `--select important|recent|modified|name`, and
the "important files first" behavior can be disabled with `--no-important`.

## Options

| Option | Description |
| --- | --- |
| `-a, --all` | Include hidden files |
| `--gitignore` | Respect `.gitignore` and hide ignored files |
| `-L, --max-depth N` | Limit tree depth |
| `-d, --dirs-only` | Directories only |
| `-f, --full-path` | Print full path prefixes |
| `--limit N` | Files shown per type per directory (default 2) |
| `--select MODE` | `important` (default), `recent`, `modified`, or `name` |
| `--sort KEY` | `name`, `mtime`, `commits`, `size`, `version`, or `none` |
| `-r, --reverse` | Reverse sort order |
| `--filesfirst` | List files before directories |
| `--commits` / `--no-commits` | Toggle last-commit annotations |
| `--recent[=N]` | Flat list of N most recently committed files |
| `-s, --size` | File sizes in bytes |
| `-h, --human` | Human-readable sizes (powers of 1024) |
| `--si` | Human-readable sizes (powers of 1000) |
| `--du` | Directory sizes as accumulation of contents |
| `-D, --date` | Last modification date |
| `--timefmt FORMAT` | strftime date format (implies `-D`) |
| `-P PATTERN` / `-I PATTERN` | Include / exclude glob patterns (repeatable) |
| `--prune` | Prune empty directories |
| `--noreport` | Omit the final report line |
| `--no-important` | Disable the important-files boost |
| `-F, --classify` | Append `/`, `*`, `@` to dirs, executables, symlinks |
| `-i, --noindent` | No indentation lines |
| `-l, --follow` | Follow symlinks to directories |
| `-C` / `-n` / `--color WHEN` | Force / disable / auto color |

Color respects the usual environment variables: `NO_COLOR` disables it and
`CLICOLOR_FORCE` forces it. `LS_COLORS` is honored when set, with a built-in
scheme as fallback.

## How it differs from `tree`

`tree` lists everything; `tow` summarizes. `tow` reuses the `tree` options that
make sense for a quick overview (`-a`, `-L`, `-d`, `-f`, `-I`, `-P`, `-h`,
`--si`, `--du`, `-D`, `--timefmt`, `-F`, `-i`, `-r`, `--prune`, `--noreport`)
and adds its own: per-type collapsing, git history annotations, and `--recent`.
Gitignored files are shown by default; pass `--gitignore` to hide them.

## Development

```sh
cargo test          # unit + integration tests
cargo clippy        # lint
```

Integration tests build throwaway directory trees (and real git repositories
with fixed commit timestamps) to verify filtering, collapsing, sizing, and
history output. See `tests/cli.rs`.
