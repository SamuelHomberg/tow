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

## Collapsing: the "exemplary files" algorithm

For each directory, files are partitioned into two classes:

1. **Important files** — a hardcoded list of recognizable project anchors
   (`README*`, `LICENSE*`, `Cargo.toml`, `package.json`, `go.mod`,
   `pyproject.toml`, `Makefile`, `Dockerfile`, …). These are always shown,
   regardless of how crowded the directory is. They are cheap to enumerate and
   are exactly the files that explain a project.
2. **Everything else** — grouped by file type (extension), each group is sorted
   and truncated to `--limit` entries (default 2). The remainder becomes a
   single summary line: `… 7 more .py`.

The sort key for step 2 defaults to **last-commit time** (falling back to
modification time, then name). This is deliberate: in a freshly cloned
repository every file has an identical `mtime`, so modification time is useless,
but git history preserves the true "last changed" signal.

The four `--select` modes are just different answers to "which files deserve the
two slots per type?":

| Mode | Sort key | Important boost |
| --- | --- | --- |
| `important` (default) | commit time → mtime → name | yes |
| `recent` | commit time → mtime → name | no |
| `modified` | mtime → name | no |
| `name` | name | no |

Directories are always shown (they are the structure) and always name-sorted,
unless `--filesfirst` or `--reverse` changes the arrangement.

## Git integration

`tow` uses `git2` (libgit2, vendored, no system dependency) rather than shelling
out to `git`.

Last-commit-per-file is computed by walking history newest-first and diffing
each commit against its first parent, stopping as soon as every requested path
has been seen. Because `tow` only needs the files it actually walks (not the
whole history), this stays cheap even on large repositories.

Commit annotations are shown **inline** after the filename
(`name.py  1a2b3c4  2026-09-20  subject`), not in an aligned column, to keep the
tree compact. Subjects are trimmed to 48 characters.

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
| `.gitignore` respected | off (opt-in via `--gitignore`) | gitignored content is shown by default so nothing is silently hidden |
| Hidden files shown | off | dotfiles are rarely what you want in an overview |
| Files per type per directory | 2 | "a couple", per the goal |
| Important-files boost | on | anchors the overview |
| Sort key | commit time | survives fresh clones |
| Directory sizes (`--du`) | off | requires reading every byte; opt-in |
| Commit annotations | on (inside a repo) | the "what changed lately" signal |
| Depth | unlimited | structure is the point; `-L` trims when needed |

## Module layout

```
src/main.rs      entry, orchestration, broken-pipe handling
src/cli.rs       clap CLI + resolved Config
src/model.rs     in-memory Node tree + git metadata
src/walk.rs      ignore-crate traversal -> tree
src/select.rs    collapsing / exemplary-file selection
src/important.rs recognized-file heuristics
src/sort.rs      natural sort + sort keys
src/git.rs       last-commit-per-file via git2
src/size.rs      human/SI formatting + --du accumulation
src/color.rs     LS_COLORS parsing + extension table
src/render.rs    tree rendering + report + --recent
```

## Trade-offs / known limitations

- `-P`/`-I` patterns match against **basenames**, not full paths.
- Directory sorting is always by name (not by `--sort`), to keep structure
  stable.
- `--du` computes sizes before collapsing, so a directory's size reflects its
  full contents even when files are hidden.
- XML/JSON/HTML output (`tree -X -J -H`) is intentionally out of scope.
