use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

/// Run `tow` in `dir` with the given args, in UTC and with color off.
fn run(dir: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("tow").unwrap();
    cmd.current_dir(dir)
        .args(args)
        .env("TZ", "UTC")
        .env("NO_COLOR", "1");
    cmd.assert()
}

fn stdout_of(dir: &Path, args: &[&str]) -> String {
    let assert = run(dir, args).success();
    String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

fn write(dir: &Path, rel: &str, content: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, content).unwrap();
}

/// Set a file's mtime to a fixed value so ordering is deterministic.
fn set_mtime(dir: &Path, rel: &str, secs: u64) {
    let p = dir.join(rel);
    let t = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs);
    let f = std::fs::OpenOptions::new().write(true).open(p).unwrap();
    f.set_times(std::fs::FileTimes::new().set_modified(t)).unwrap();
}

fn git(dir: &Path, args: &[&str], envs: &[(&str, &str)]) {
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(dir)
        .args(["-c", "user.name=Tester", "-c", "user.email=t@example.com"])
        .args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn init_repo(dir: &Path) {
    git(dir, &["init", "-q"], &[]);
}

fn commit(dir: &Path, msg: &str, iso_date: &str) {
    git(dir, &["add", "-A"], &[]);
    git(
        dir,
        &["commit", "-q", "-m", msg],
        &[
            ("GIT_AUTHOR_DATE", iso_date),
            ("GIT_COMMITTER_DATE", iso_date),
        ],
    );
}

#[test]
fn basic_tree_with_type_caps() {
    let d = TempDir::new().unwrap();
    let dir = d.path();

    write(dir, "README.md", "readme");
    write(dir, "Cargo.toml", "[package]");
    for f in ["main.rs", "lib.rs", "one.py", "two.py", "three.py", "four.py", "five.py"] {
        write(dir, &format!("src/{f}"), "");
    }

    // Same mtime everywhere so ordering falls back to name.
    for rel in [
        "README.md",
        "Cargo.toml",
        "src/main.rs",
        "src/lib.rs",
        "src/one.py",
        "src/two.py",
        "src/three.py",
        "src/four.py",
        "src/five.py",
    ] {
        set_mtime(dir, rel, 100);
    }

    insta::assert_snapshot!(stdout_of(dir, &["."]));
}

#[test]
fn gitignored_files_shown_by_default() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, ".gitignore", "node_modules/\nbuild/\n");
    write(dir, "README.md", "readme");
    write(dir, "node_modules/pkg/index.js", "");
    write(dir, "build/out.o", "");
    write(dir, "src/main.rs", "");

    // By default gitignored files are shown.
    insta::assert_snapshot!(stdout_of(dir, &["."]));

    // --gitignore hides them (but -a still reveals hidden dotfiles).
    let out = stdout_of(dir, &["--gitignore", "."]);
    assert!(!out.contains("node_modules"), "gitignored shown:\n{out}");
    assert!(!out.contains("build"), "gitignored shown:\n{out}");
    assert!(out.contains("README.md"), "missing README.md:\n{out}");

    // -a reveals the hidden .gitignore file itself.
    let out = stdout_of(dir, &["-a", "."]);
    assert!(out.contains(".gitignore"), "missing .gitignore:\n{out}");
}

#[test]
fn max_depth_and_dirs_only() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, "a/b/c/deep.txt", "");
    write(dir, "a/top.txt", "");

    insta::assert_snapshot!(stdout_of(dir, &["-L", "1", "."]));
    insta::assert_snapshot!(stdout_of(dir, &["-d", "."]));
}

#[test]
fn du_reports_directory_sizes() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, "src/a.rs", "12345"); // 5 bytes
    write(dir, "src/b.rs", "1234567890"); // 10 bytes
    write(dir, "top.txt", "123"); // 3 bytes

    insta::assert_snapshot!(stdout_of(dir, &["--du", "-s", "."]));
}

#[test]
fn include_and_exclude_patterns() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, "src/a.rs", "");
    write(dir, "src/b.py", "");
    write(dir, "src/c.rs", "");

    insta::assert_snapshot!(stdout_of(dir, &["-P", "*.rs", "."]));
    insta::assert_snapshot!(stdout_of(dir, &["-I", "*.py", "."]));
}

#[test]
fn commits_are_annotated() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    init_repo(dir);

    write(dir, "main.rs", "fn main(){}");
    commit(dir, "init: add main", "2026-01-01T00:00:00+00:00");
    write(dir, "src/lib.rs", "pub fn x(){}");
    commit(dir, "feat: add lib", "2026-01-02T00:00:00+00:00");
    write(dir, "README.md", "readme");
    commit(dir, "docs: readme", "2026-01-03T00:00:00+00:00");

    let out = stdout_of(dir, &["."]);
    assert!(out.contains("2026-01-02 00:00  feat: add lib"), "got:\n{out}");
    assert!(out.contains("2026-01-03 00:00  docs: readme"), "got:\n{out}");
    assert!(out.contains("2026-01-01 00:00  init: add main"), "got:\n{out}");
    assert!(out.contains("1 directory, 3 files"), "got:\n{out}");
}

#[test]
fn commit_order_survives_fresh_clone() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    init_repo(dir);

    write(dir, "src/a.rs", "");
    commit(dir, "add a", "2026-01-01T00:00:00+00:00");
    write(dir, "src/b.rs", "");
    commit(dir, "add b", "2026-01-03T00:00:00+00:00");
    write(dir, "src/c.rs", "");
    commit(dir, "add c", "2026-01-02T00:00:00+00:00");

    // Fresh clone: identical mtimes on every file.
    for rel in ["src/a.rs", "src/b.rs", "src/c.rs"] {
        set_mtime(dir, rel, 500);
    }

    // The most recently committed file (b.rs) must surface even though the
    // filesystem mtimes are all equal. --no-commits keeps output deterministic.
    insta::assert_snapshot!(stdout_of(dir, &["--no-commits", "--limit", "1", "."]));
}

#[test]
fn recent_flat_list() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    init_repo(dir);

    write(dir, "old.rs", "");
    commit(dir, "old", "2026-01-01T00:00:00+00:00");
    write(dir, "new.rs", "");
    commit(dir, "new", "2026-01-02T00:00:00+00:00");

    let out = stdout_of(dir, &["--recent=1", "."]);
    assert!(out.contains("2026-01-02 00:00  new.rs  new"), "got:\n{out}");
    assert!(!out.contains("old.rs"), "got:\n{out}");
}

#[test]
fn color_modes() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, "a.rs", "");

    let assert = run(dir, &["-C", "."]).success();
    let bytes = assert.get_output().stdout.clone();
    assert!(String::from_utf8_lossy(&bytes).contains("\x1b["));

    let assert = run(dir, &["-n", "."]).success();
    let bytes = assert.get_output().stdout.clone();
    assert!(!String::from_utf8_lossy(&bytes).contains("\x1b["));
}

#[test]
fn entrypoints_surface_first() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, "src/main.rs", "");
    for f in ["a.py", "b.py", "c.py"] {
        write(dir, &format!("src/{f}"), "");
    }

    let out = stdout_of(dir, &["."]);
    let main = out.find("main.rs").expect("main.rs missing");
    let py = out.find(".py").expect(".py missing");
    assert!(main < py, "main.rs should precede .py files:\n{out}");
}

#[test]
fn config_file_overrides_rules() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, ".tow.toml", "[files]\nentrypoints = [\"custom_*\"]\n");
    write(dir, "src/custom_thing.py", "");
    write(dir, "src/other.py", "");

    let out = stdout_of(dir, &["."]);
    let c = out.find("custom_thing.py").expect("custom_thing.py missing");
    let o = out.find("other.py").expect("other.py missing");
    assert!(c < o, "custom_thing.py should be surfaced first:\n{out}");

    // --no-config ignores the file, so alphabetical order wins.
    let out = stdout_of(dir, &["--no-config", "."]);
    let c = out.find("custom_thing.py").expect("custom_thing.py missing");
    let o = out.find("other.py").expect("other.py missing");
    assert!(c < o, "both are ordinary files now:\n{out}");
}

#[test]
fn height_collapses_directories() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, "src/main.rs", "");
    write(dir, "node_modules/a/b/c.js", "");
    write(dir, "node_modules/d.js", "");

    let out = stdout_of(dir, &["--height", "3", "."]);
    assert!(out.contains("directories hidden"), "got:\n{out}");
    assert!(!out.contains("node_modules"), "noise dir should be hidden:\n{out}");
    assert!(out.contains("src"), "protected dir should stay:\n{out}");

    let out = stdout_of(dir, &["--all-dirs", "."]);
    assert!(out.contains("node_modules"), "got:\n{out}");
}

#[test]
fn hidden_dirs_star_shape() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, "src/main.rs", "");
    for i in 0..5 {
        write(dir, &format!("vendor/sub{i}/placeholder.txt"), "");
    }
    let out = stdout_of(dir, &["--height", "4", "."]);
    assert!(out.contains("one directory, 5 subdirs"), "got:\n{out}");
}

#[test]
fn hidden_dirs_chain_shape() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, "src/main.rs", "");
    write(dir, "vendor/a/b/c/d/f.txt", "");
    let out = stdout_of(dir, &["--height", "4", "."]);
    assert!(out.contains("a chain"), "got:\n{out}");
}

#[test]
fn hidden_dirs_multiple_roots() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, "src/main.rs", "");
    write(dir, "node_modules/a/f.txt", "");
    write(dir, "build/b/f.txt", "");
    let out = stdout_of(dir, &["--height", "4", "."]);
    assert!(out.contains("2 roots"), "got:\n{out}");
}

#[test]
fn size_flags_aggregate_directory_sizes() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, "src/a.rs", "12345"); // 5 bytes
    write(dir, "src/b.rs", "1234567890"); // 10 bytes

    let out = stdout_of(dir, &["-s", "."]);
    assert!(out.contains("15 src"), "directory size should aggregate:\n{out}");
    assert!(out.contains("5 a.rs"), "got:\n{out}");
    assert!(out.contains("10 b.rs"), "got:\n{out}");
}

#[test]
fn dump_config_prints_toml() {
    let d = TempDir::new().unwrap();
    let dir = d.path();

    let out = stdout_of(dir, &["--dump-config"]);
    assert!(out.contains("[files]"), "got:\n{out}");
    assert!(out.contains("entrypoints"), "got:\n{out}");
    assert!(out.contains("[dirs]"), "got:\n{out}");
    assert!(out.contains("[display]"), "got:\n{out}");
    assert!(out.contains("[defaults]"), "got:\n{out}");
}

#[test]
fn commit_annotation_truncates_to_width() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    init_repo(dir);
    write(dir, "main.rs", "");
    commit(dir, "implement the main feature", "2026-01-01T00:00:00+00:00");

    let full = stdout_of(dir, &["--full-commits", "."]);
    assert!(
        full.contains("implement the main feature"),
        "got:\n{full}"
    );

    let narrow = stdout_of(dir, &["--width", "25", "."]);
    assert!(narrow.contains("…"), "expected truncation:\n{narrow}");
}

#[test]
fn config_gitignore_toggle() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, ".tow.toml", "[display]\ngitignore = true\n");
    write(dir, ".gitignore", "build/\n");
    write(dir, "build/out.o", "");
    write(dir, "src/main.rs", "");

    // Config says hide → build/ is hidden by default.
    let out = stdout_of(dir, &["."]);
    assert!(!out.contains("build"), "gitignored should be hidden:\n{out}");

    // --no-gitignore overrides the config.
    let out = stdout_of(dir, &["--no-gitignore", "."]);
    assert!(out.contains("build"), "got:\n{out}");
}

#[test]
fn config_defaults_limit() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, ".tow.toml", "[defaults]\nlimit = 1\n");
    for f in ["a.py", "b.py", "c.py"] {
        write(dir, &format!("src/{f}"), "");
        set_mtime(dir, &format!("src/{f}"), 100);
    }
    let out = stdout_of(dir, &["."]);
    assert!(out.contains("… 2 more .py"), "got:\n{out}");
}

#[test]
fn cli_overrides_config_defaults() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, ".tow.toml", "[defaults]\nlimit = 1\n");
    for f in ["a.py", "b.py", "c.py"] {
        write(dir, &format!("src/{f}"), "");
        set_mtime(dir, &format!("src/{f}"), 100);
    }
    let out = stdout_of(dir, &["--limit", "2", "."]);
    assert!(out.contains("… 1 more .py"), "got:\n{out}");
}

#[test]
fn config_defaults_max_depth() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, ".tow.toml", "[defaults]\nmax_depth = 1\n");
    write(dir, "a/b/c/deep.txt", "");
    let out = stdout_of(dir, &["."]);
    assert!(!out.contains("deep.txt"), "depth should be limited:\n{out}");
    assert!(out.contains("a"), "got:\n{out}");
}

#[test]
fn config_defaults_height_forces_collapse() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    write(dir, ".tow.toml", "[defaults]\nheight = 3\n");
    write(dir, "src/main.rs", "");
    write(dir, "node_modules/a/b.js", "");
    // Piped output normally never collapses; the config height forces it.
    let out = stdout_of(dir, &["."]);
    assert!(out.contains("directories hidden"), "got:\n{out}");
}

#[test]
fn max_files_cap_truncates() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    for i in 0..10 {
        write(dir, &format!("f{i}.txt"), "");
    }

    let assert = run(dir, &["--max-files", "3", "."]).success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("--max-files"), "got stderr:\n{stderr}");
    assert!(stderr.contains("incomplete"), "got stderr:\n{stderr}");
}

#[test]
fn max_commits_degrades_gracefully() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    init_repo(dir);
    write(dir, "a.rs", "");
    commit(dir, "first", "2026-01-01T00:00:00+00:00");
    write(dir, "b.rs", "");
    commit(dir, "second", "2026-01-02T00:00:00+00:00");

    // Only the newest commit is scanned, so a.rs has no annotation.
    let out = stdout_of(dir, &["--max-commits", "1", "."]);
    assert!(out.contains("second"), "got:\n{out}");
    assert!(!out.contains("first"), "got:\n{out}");
}
