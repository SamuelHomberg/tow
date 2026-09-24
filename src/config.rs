use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::Cli;

/// The merged, fully-resolved rule set (defaults + user config + project
/// config). This is what `--dump-config` prints and what drives classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rules {
    pub files: FilesRules,
    pub dirs: DirsRules,
    pub display: DisplayRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesRules {
    pub entrypoints: Vec<String>,
    pub important: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirsRules {
    pub protect: Vec<String>,
    pub noise: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayRules {
    pub collapse: bool,
    pub gitignore: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            files: FilesRules {
                entrypoints: vec![
                    "main.*".into(),
                    "index.*".into(),
                    "app.*".into(),
                    "server.*".into(),
                    "lib.*".into(),
                    "cli.*".into(),
                    "manage.py".into(),
                    "wsgi.py".into(),
                    "asgi.py".into(),
                    "setup.py".into(),
                ],
                important: vec![
                    "README*".into(),
                    "LICENSE*".into(),
                    "LICENCE*".into(),
                    "COPYING*".into(),
                    "CHANGELOG*".into(),
                    "AUTHORS*".into(),
                    "CONTRIBUTING*".into(),
                    "Makefile".into(),
                    "makefile".into(),
                    "Justfile".into(),
                    "justfile".into(),
                    "Cargo.toml".into(),
                    "Cargo.lock".into(),
                    "package.json".into(),
                    "package-lock.json".into(),
                    "go.mod".into(),
                    "go.sum".into(),
                    "pyproject.toml".into(),
                    "requirements.txt".into(),
                    "pom.xml".into(),
                    "build.gradle".into(),
                    "build.gradle.kts".into(),
                    "settings.gradle".into(),
                    "Dockerfile".into(),
                    "docker-compose.yml".into(),
                    "docker-compose.yaml".into(),
                    "compose.yml".into(),
                    "compose.yaml".into(),
                    "Gemfile".into(),
                    "CMakeLists.txt".into(),
                    "meson.build".into(),
                    ".gitignore".into(),
                    ".gitattributes".into(),
                    ".editorconfig".into(),
                    "rust-toolchain*".into(),
                    "deno.json".into(),
                ],
            },
            dirs: DirsRules {
                protect: vec![
                    "src".into(),
                    "lib".into(),
                    "app".into(),
                    "cmd".into(),
                    "internal".into(),
                    "pkg".into(),
                    "include".into(),
                    "tests".into(),
                    "test".into(),
                    "docs".into(),
                    "doc".into(),
                    "examples".into(),
                    "example".into(),
                    "bench".into(),
                    "benches".into(),
                    "benchmarks".into(),
                    "scripts".into(),
                    "tools".into(),
                ],
                noise: vec![
                    "node_modules".into(),
                    "target".into(),
                    "build".into(),
                    "dist".into(),
                    "out".into(),
                    "vendor".into(),
                    "coverage".into(),
                    "__pycache__".into(),
                    ".git".into(),
                    ".venv".into(),
                    "venv".into(),
                    ".tox".into(),
                    ".eggs".into(),
                    ".idea".into(),
                    ".vscode".into(),
                    ".gradle".into(),
                    ".mvn".into(),
                    "debug".into(),
                    "release".into(),
                    ".pytest_cache".into(),
                    ".mypy_cache".into(),
                    ".ruff_cache".into(),
                ],
            },
            display: DisplayRules {
                collapse: true,
                gitignore: false,
            },
        }
    }
}

// Partial forms used for merging: every key is optional so a config file can
// override just one rule without resetting the rest.
#[derive(Debug, Default, Deserialize)]
struct PartialRules {
    #[serde(default)]
    files: PartialFiles,
    #[serde(default)]
    dirs: PartialDirs,
    #[serde(default)]
    display: PartialDisplay,
}

#[derive(Debug, Default, Deserialize)]
struct PartialFiles {
    #[serde(default)]
    entrypoints: Option<Vec<String>>,
    #[serde(default)]
    important: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
struct PartialDirs {
    #[serde(default)]
    protect: Option<Vec<String>>,
    #[serde(default)]
    noise: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
struct PartialDisplay {
    #[serde(default)]
    collapse: Option<bool>,
    #[serde(default)]
    gitignore: Option<bool>,
}

impl Rules {
    fn apply(&mut self, p: &PartialRules) {
        if let Some(v) = &p.files.entrypoints {
            self.files.entrypoints = v.clone();
        }
        if let Some(v) = &p.files.important {
            self.files.important = v.clone();
        }
        if let Some(v) = &p.dirs.protect {
            self.dirs.protect = v.clone();
        }
        if let Some(v) = &p.dirs.noise {
            self.dirs.noise = v.clone();
        }
        if let Some(v) = p.display.collapse {
            self.display.collapse = v;
        }
        if let Some(v) = p.display.gitignore {
            self.display.gitignore = v;
        }
    }
}

/// Load and merge the effective rules: hardcoded defaults, then the user
/// config, then the project `.tow.toml`.
pub fn load(cli: &Cli) -> Result<Rules> {
    let mut rules = Rules::default();
    if cli.no_config {
        return Ok(rules);
    }

    if let Some(path) = user_config_path(cli)
        && path.exists()
    {
        let partial = parse_file(&path)?;
        rules.apply(&partial);
    }

    if let Some(path) = find_project_config() {
        let partial = parse_file(&path)?;
        rules.apply(&partial);
    }

    Ok(rules)
}

fn parse_file(path: &Path) -> Result<PartialRules> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("failed to parse config file {}", path.display()))
}

/// Resolve the user config file: `--config PATH`, then `$TOW_CONFIG`, then the
/// XDG default (`~/.config/tow/config.toml`).
fn user_config_path(cli: &Cli) -> Option<PathBuf> {
    if let Some(p) = &cli.config {
        return Some(p.clone());
    }
    if let Some(p) = std::env::var_os("TOW_CONFIG")
        && !p.is_empty()
    {
        return Some(PathBuf::from(p));
    }
    user_config_dir().map(|d| d.join("config.toml"))
}

/// The XDG config directory for tow (e.g. `~/.config/tow`), or `None` if the
/// platform provides no config directory.
fn user_config_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|base| base.config_dir().join("tow"))
}

/// Walk up from the current directory looking for a project `.tow.toml`.
fn find_project_config() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(".tow.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Serialize the merged rules as TOML (used by `--dump-config`).
pub fn dump(rules: &Rules) -> String {
    toml::to_string_pretty(rules).unwrap_or_else(|_| "# failed to serialize config\n".to_string())
}
