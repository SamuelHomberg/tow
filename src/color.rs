use std::collections::HashMap;
use std::io::IsTerminal;

use crate::cli::ColorMode;
use crate::model::Node;

const RESET: &str = "\x1b[0m";

/// Wrap `text` in an ANSI SGR sequence. `sgr` is e.g. "1;34" (no escape or 'm').
fn paint(text: &str, sgr: &str) -> String {
    if sgr.is_empty() {
        text.to_string()
    } else {
        format!("\x1b[{sgr}m{text}{RESET}")
    }
}

/// A consistent extension -> SGR code table. The specific colors are
/// arbitrary; what matters is that they are stable across runs.
pub fn ext_color(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "rs" => "1;33",
        "py" | "pyw" | "pyi" => "1;34",
        "js" | "jsx" | "mjs" | "cjs" => "1;33",
        "ts" | "tsx" | "mts" | "cts" => "1;34",
        "go" => "1;36",
        "c" | "h" => "1;36",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => "1;36",
        "java" => "1;31",
        "rb" => "1;31",
        "php" => "1;35",
        "sh" | "bash" | "zsh" | "fish" => "1;32",
        "toml" => "1;31",
        "json" | "jsonc" | "json5" => "1;33",
        "yaml" | "yml" => "1;33",
        "xml" => "1;33",
        "md" | "markdown" | "rst" | "adoc" => "1;36",
        "txt" | "text" => "37",
        "html" | "htm" => "1;31",
        "css" => "1;35",
        "scss" | "sass" | "less" => "1;35",
        "sql" => "1;35",
        "lock" => "90",
        "nix" => "1;36",
        "lua" => "1;34",
        "vim" => "1;32",
        "ps1" | "psm1" => "1;36",
        "ex" | "exs" => "1;35",
        "erl" | "hrl" => "1;35",
        "hs" | "lhs" => "1;35",
        "scala" => "1;31",
        "kt" | "kts" => "1;35",
        "swift" => "1;31",
        "r" => "1;34",
        "dart" => "1;36",
        "zig" => "1;33",
        "ml" | "mli" => "1;35",
        "pl" | "pm" => "1;36",
        "fs" | "fsx" | "fsi" => "1;36",
        "cs" => "1;36",
        "vue" | "svelte" => "1;32",
        "gradle" => "1;36",
        "ini" | "conf" | "cfg" | "env" => "90",
        "svg" | "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" => "1;35",
        "pdf" => "1;31",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "zst" | "rar" | "7z" => "1;31",
        _ => return None,
    })
}

/// Parsed `LS_COLORS` environment variable (numeric SGR values as emitted by
/// `dircolors`).
#[derive(Debug, Default)]
pub struct LsColors {
    map: HashMap<String, String>,
}

impl LsColors {
    pub fn from_env() -> Option<LsColors> {
        let raw = std::env::var("LS_COLORS").ok()?;
        let mut map = HashMap::new();
        for part in raw.split(':') {
            if let Some((k, v)) = part.split_once('=') {
                map.insert(k.to_string(), v.to_string());
            }
        }
        Some(LsColors { map })
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|s| s.as_str())
    }
}

/// Responsible for deciding whether and how to colorize output.
pub struct Painter {
    pub enabled: bool,
    ls: Option<LsColors>,
}

impl Painter {
    pub fn new(mode: ColorMode) -> Painter {
        let enabled = match mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => std::io::stdout().is_terminal(),
        };
        let ls = if enabled { LsColors::from_env() } else { None };
        Painter { enabled, ls }
    }

    fn ls_sgr(&self, key: &str) -> Option<String> {
        self.ls.as_ref().and_then(|ls| ls.get(key).map(str::to_string))
    }

    fn category(&self, key: &str, fallback: &'static str) -> String {
        self.ls_sgr(key).unwrap_or_else(|| fallback.to_string())
    }

    /// Colorize a node name, splitting stem and extension so the extension
    /// can be highlighted in its own consistent color.
    pub fn name(&self, node: &Node) -> String {
        let name = node.name();
        if !self.enabled {
            return name.to_string();
        }

        let base = if node.is_dir() {
            self.category("di", "1;34")
        } else if node.is_symlink() {
            self.category("ln", "1;36")
        } else if node.is_exec() {
            self.category("ex", "1;32")
        } else {
            self.category("fi", "")
        };

        let (stem, ext) = split_ext(name);
        let mut out = String::new();
        out.push_str(&paint(stem, &base));

        if let Some(ext) = ext {
            let lower = ext.to_ascii_lowercase();
            let ext_sgr = ext_color(&lower)
                .map(str::to_string)
                .or_else(|| self.ls_sgr(&format!("*.{lower}")));
            let ext_text = format!(".{ext}");
            match ext_sgr {
                Some(sgr) => out.push_str(&paint(&ext_text, &sgr)),
                None => out.push_str(&paint(&ext_text, &base)),
            }
        }

        out
    }

    /// Colorize a directory's name (root line and path headers).
    pub fn dir(&self, name: &str) -> String {
        if !self.enabled {
            return name.to_string();
        }
        paint(name, &self.category("di", "1;34"))
    }

    /// Colorize a full path: directories use the dir color, files stay plain.
    pub fn path(&self, name: &str, node: &Node) -> String {
        if !self.enabled {
            return name.to_string();
        }
        let base = if node.is_dir() {
            self.category("di", "1;34")
        } else {
            String::new()
        };
        paint(name, &base)
    }

    /// Muted color for the "… N more" summary lines.
    pub fn muted(&self, text: &str) -> String {
        if !self.enabled {
            return text.to_string();
        }
        paint(text, "90")
    }

    /// Muted color for git hash/date/subject annotations.
    pub fn meta(&self, text: &str) -> String {
        if !self.enabled {
            return text.to_string();
        }
        paint(text, "90")
    }
}

/// Split a file name into (stem, Some(extension)). Leading-dot names and
/// names without a dot yield `(name, None)`.
fn split_ext(name: &str) -> (&str, Option<&str>) {
    let base = name.rsplit('/').next().unwrap_or(name);
    match base.rfind('.') {
        Some(i) if i > 0 => (&base[..i], Some(&base[i + 1..])),
        _ => (base, None),
    }
}

#[cfg(test)]
mod tests {
    use super::split_ext;

    #[test]
    fn split_extension() {
        assert_eq!(split_ext("main.py"), ("main", Some("py")));
        assert_eq!(split_ext("foo.tar.gz"), ("foo.tar", Some("gz")));
        assert_eq!(split_ext("Makefile"), ("Makefile", None));
        assert_eq!(split_ext(".gitignore"), (".gitignore", None));
        assert_eq!(split_ext("src/main.rs"), ("main", Some("rs")));
    }
}
