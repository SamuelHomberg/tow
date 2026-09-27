# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this
project adheres to [Semantic Versioning](https://semver.org/).

## [0.4.0] - 2026-09-27

### Added
- `[defaults]` config section supplying defaults for `max_depth`, `height`,
  `width`, `limit`, `max_files`, `max_dirs`, and `max_commits`. An explicit
  command-line flag still overrides the configured value.

## [0.3.1] - 2026-09-24

### Changed
- The `… N directories hidden` summary now describes the *shape* of the hidden
  tree (number of roots, max depth, max fan-out), with friendly wording for
  pure chains and flat stars — so `200 subdirs of one directory` is
  distinguishable from `10 roots × 20 subdirs`.

## [0.3.0] - 2026-09-24

### Added
- `[display] gitignore` config toggle for hiding `.gitignore`-ignored files by
  default; `--gitignore` / `--no-gitignore` override it.
- Safeguard cutoffs for huge projects: `--max-files` (50,000), `--max-dirs`
  (10,000), and `--max-commits` (50,000), each `0` = unlimited.

### Changed
- `__init__.py` and `mod.rs` are no longer treated as entrypoints (they are
  package/module markers, not entry points).
- Screen-fit collapse now runs in linear time via precomputed subtree metrics.

## [0.2.0] - 2026-09-24

### Added
- Priority tiers: entrypoints (`main.rs`, `index.js`, `__init__.py`, …) always
  surface first; project anchors (`README`, manifests, …) always shown.
- Directory classes: protected dirs (`src`, `tests`, `docs`, …) sort first and
  are never dropped; noise dirs (`node_modules`, `target`, …) sort last.
- Screen-fit collapsing on interactive terminals with a
  `… N directories hidden` summary; `--all-dirs` and `--height N` to control it.
- Terminal-width-aware commit truncation (drops hash → date → subject), with
  `--full-commits` to disable.
- Configurable rule sets: `~/.config/tow/config.toml` + project `.tow.toml`,
  with `--config`, `TOW_CONFIG`, `--no-config`, and `--dump-config`.

### Changed
- `-s` / `-h` / `--si` now aggregate and display directory sizes (not just
  `--du`).

## [0.1.0] - 2026-09-24

### Added
- Directory tree rendering with a "quick overview" focus.
- Gitignored files/directories shown by default; `--gitignore` hides them.
- Per-type file collapsing with a `… N more .ext` summary (`--limit`).
- "Important files" always surfaced (`--no-important` to disable).
- Four selection strategies via `--select important|recent|modified|name`.
- Sorting by name, mtime, commit time, size, version, or none (`--sort`).
- Git history integration via `git2`: last-commit annotations (`--commits` /
  `--no-commits`) and a `--recent` flat list.
- Directory sizes via `--du`, plus `-s` / `-h` / `--si` size display.
- File-type coloring with a consistent extension→color table, honoring
  `LS_COLORS`.
- Ported `tree` options: `-a`, `-L`, `-d`, `-f`, `-I`, `-P`, `-h`, `--si`,
  `-D`, `--timefmt`, `-F`, `-i`, `-l`, `-r`, `--prune`, `--noreport`.
- Unit and integration tests (including real git repositories with fixed
  commit timestamps).
