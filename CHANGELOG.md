# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this
project adheres to [Semantic Versioning](https://semver.org/).

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
