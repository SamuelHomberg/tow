# Design

This document records the key design decisions behind `tow`, so they survive
code churn and future contributors can see the "why".

## Goal

`tow` answers one question fast: *"what is this project and what's been going on
lately?"* It is deliberately **not** a complete directory lister — `tree`,
`ls`, and `find` already do that. Everything in `tow` optimizes for a glanceable
overview of a repository you have never seen (or have forgotten).

## Core idea: build the whole tree, then prune

`tow` walks the entire directory tree into memory first, then collapses it. This
is the opposite of `tree --filelimit`, which stops descending as soon as a
directory is "full". Building first means:

- Very **deep** and very **flat** repositories are both represented well.
- The "how many files were hidden" summary can be computed exactly, per type.
- Ordering decisions (e.g. "most recently committed first") can see the whole
  directory before choosing what to show.

The cost is memory proportional to the number of entries, which is fine for the
project-sized trees `tow` targets.

## Priority tiers (files)

Each file is assigned one of three tiers by a configurable rule set:

1. **Entrypoint** (`main.*`, `index.*`, `app.*`, `lib.*`, `cli.*`, `manage.py`,
   `wsgi.py`, `asgi.py`, `setup.py`, …) — always shown, listed first. This fixes
   the classic failure mode where `main.rs` gets crowded out of a busy `src/`.
   Package/module markers (`__init__.py`, `mod.rs`) are deliberately **not**
   entrypoints: they appear in every package directory and add noise, not signal.
2. **Anchor** (`README*`, `LICENSE*`, `Cargo.toml`, `package.json`, `go.mod`,
   `pyproject.toml`, `Makefile`, `Dockerfile`, …) — always shown (the "important
   files" boost; disable with `--no-important`).
3. **Ordinary** — grouped by file type, sorted, and truncated to `--limit`
   entries (default 2) per type. The remainder becomes `… 7 more .py`.

The sort key for tier 3 defaults to **last-commit time** (falling back to
modification time, then name). This is deliberate: in a freshly cloned
repository every file has an identical `mtime`, so modification time is useless,
but git history preserves the true "last changed" signal.

The four `--select` modes are different answers to "which files deserve the two
slots per type?":

| Mode | Sort key | Anchor boost |
| --- | --- | --- |
| `important` (default) | commit time → mtime → name | yes |
| `recent` | commit time → mtime → name | no |
| `modified` | mtime → name | no |
| `name` | name | no |

## Directory classes (sorting + collapsing)

Directories are classified three ways, in ascending order of interestingness:

- **Protected** (`src`, `tests`, `docs`, …) — sorted first, never dropped.
- **Normal** — sorted in the middle.
- **Noise** (`node_modules`, `target`, `build`, …) — sorted last, hidden first
  when the tree exceeds the screen.

This classification drives both **ordering** (within a directory) and **screen
collapse** (below).

## Screen-fit collapse

On an interactive terminal, `tow` tries to keep the overview within one screen.

The renderer walks the tree top-down with a line budget. A non-protected
directory is shown only if its whole subtree fits in the remaining budget;
otherwise it is hidden entirely and its directories are counted. Protected
directories are always rendered in full.

The hidden remainder is summarized with its **shape**, not just its size, so a
reader can tell a wide-flat tree from a deep-narrow one. Two graph-theoretic
metrics are precomputed per directory (in the same bottom-up pass as height and
dir count):

- **max depth** (tree *height*: longest root→leaf path),
- **max fan-out** (*maximum degree*: most subdirectories of any one directory).

Aggregated over the hidden forest, they produce:

```
… N directories hidden (a chain N deep)                    # max fan-out ≤ 1
… N directories hidden (one directory, M subdirs)          # depth ≤ 2
… N directories hidden (≤W subdirs, D deep)                # single root, general
… N directories hidden (R roots, ≤W subdirs, D deep)       # multiple roots
```

- `--all-dirs` disables collapsing entirely.
- `--height N` sets an explicit budget (and forces collapsing even when output
  is piped).
- Piped output (non-tty) never collapses by default, so `tow > file` does not
  drop data.

Known trade-off: a *protected* directory larger than the budget still renders in
full (it is the point of the overview), so the "one screen" goal is
best-effort when the interesting content itself is large.

## Configuration

Rules (entrypoints, anchors, protected/noise dirs, collapse, gitignore) are
softcoded and merged from three layers, lowest precedence first:

1. Built-in defaults.
2. User config: `--config PATH` → `$TOW_CONFIG` →
   `$XDG_CONFIG_HOME/tow/config.toml`.
3. Project `.tow.toml` (walk up from the current directory).

`--no-config` ignores all files; `--dump-config` prints the merged result as
TOML. Patterns are `globset` globs matched against basenames.

The `[display] gitignore` key controls whether `.gitignore`-ignored files are
hidden by default; the `--gitignore` / `--no-gitignore` flags override it.

## Safeguards against huge projects

Three cutoffs bound the cost of walking projects with millions of entries (each
`0` = unlimited):

- `--max-files` (default 50,000) — stops building the tree once the file cap is
  hit; a notice is printed and the tree is reported incomplete.
- `--max-dirs` (default 10,000) — same for directories.
- `--max-commits` (default 50,000) — bounds the git history walk; files whose
  last commit is older than the scanned window simply fall back to `mtime`.

Additionally, each directory's rendered height and directory count are
precomputed in a single bottom-up pass after pruning, so the screen-fit
collapse runs in linear time rather than O(N·depth) on deep trees.

## Git integration

`tow` uses `git2` (libgit2, vendored, no system dependency) rather than shelling
out to `git`.

Last-commit-per-file is computed by walking history newest-first and diffing
each commit against its first parent, stopping as soon as every requested path
has been seen. Because `tow` only needs the files it actually walks (not the
whole history), this stays cheap even on large repositories.

Commit annotations are shown **inline** after the filename
(`name.py  1a2b3c4  2026-09-20  subject`), not in an aligned column, to keep the
tree compact. Subjects are first trimmed to 48 characters, then further
truncated to the terminal width — dropping the hash, then the date, then the
subject, in that order. `--full-commits` disables width truncation.

## Colors

- Filename categories (directory, symlink, executable) use `LS_COLORS` when set,
  with a built-in fallback.
- File **extensions** get their own consistent color from a static table
  (`.py` = blue, `.rs` = orange, `.toml` = red, …). The specific colors are
  arbitrary; the consistency is what matters. The extension is split from the
  stem and painted separately so `.py` always appears in "the Python color".

## Defaults

| Setting | Default | Rationale |
| --- | --- | --- |
| `.gitignore` respected | configurable (`[display] gitignore`, default off) | nothing is silently hidden unless opted in |
| Hidden files shown | off | dotfiles are rarely what you want in an overview |
| Files per type per directory | 2 | "a couple", per the goal |
| Anchor boost | on | anchors the overview |
| Entrypoints shown | always | `main.rs` must never vanish |
| Sort key | commit time | survives fresh clones |
| Directory sizes | off (any of `-s`/`-h`/`--si`) | requires reading every byte; opt-in |
| Screen-fit collapse | on (tty only) | the overview should fit one screen |
| Commit annotations | on (inside a repo) | the "what changed lately" signal |
| Depth | unlimited | structure is the point; `-L` trims when needed |
| File / dir / commit caps | 50k / 10k / 50k | bound huge projects; `0` = unlimited |

## Module layout

```
src/main.rs      entry, orchestration, terminal-size resolution, broken-pipe
src/cli.rs       clap CLI + resolved Config
src/config.rs    rule discovery/merge/parse + --dump-config
src/priority.rs  compiled rule sets: file tiers + directory classes
src/model.rs     in-memory Node tree + git metadata
src/walk.rs      ignore-crate traversal -> tree
src/select.rs    collapsing / tier-based file selection
src/sort.rs      natural sort + sort keys
src/git.rs       last-commit-per-file via git2
src/size.rs      human/SI formatting + directory accumulation
src/color.rs     LS_COLORS parsing + extension table
src/render.rs    tree rendering, budget collapse, width truncation, report
```

## Trade-offs / known limitations

- `-P`/`-I` patterns match against **basenames**, not full paths.
- Directory sorting is by class then name (not `--sort`), to keep structure
  stable.
- Sizes are computed before collapsing, so a directory's size reflects its full
  contents even when files are hidden.
- A protected directory larger than the screen budget still renders in full.
- XML/JSON/HTML output (`tree -X -J -H`) is intentionally out of scope.
