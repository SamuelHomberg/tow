use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::config::Rules;

/// Directory priority class, in ascending order of interestingness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirClass {
    /// Core source directories; never hidden, sorted first.
    Protected,
    /// Ordinary directories.
    Normal,
    /// Generated/vendored/cache directories; sorted last, hidden first.
    Noise,
}

/// Compiled glob matchers for the rule lists, used on the hot path.
pub struct RuleSet {
    entrypoints: GlobSet,
    important: GlobSet,
    protect: GlobSet,
    noise: GlobSet,
}

impl RuleSet {
    pub fn compile(rules: &Rules) -> Result<RuleSet> {
        Ok(RuleSet {
            entrypoints: build(&rules.files.entrypoints)?,
            important: build(&rules.files.important)?,
            protect: build(&rules.dirs.protect)?,
            noise: build(&rules.dirs.noise)?,
        })
    }

    /// 0 = entrypoint, 1 = anchor/important, 2 = ordinary.
    pub fn file_tier(&self, name: &str) -> u8 {
        if self.entrypoints.is_match(name) {
            0
        } else if self.important.is_match(name) {
            1
        } else {
            2
        }
    }

    pub fn dir_class(&self, name: &str) -> DirClass {
        if self.protect.is_match(name) {
            DirClass::Protected
        } else if self.noise.is_match(name) {
            DirClass::Noise
        } else {
            DirClass::Normal
        }
    }

    pub fn is_protected_dir(&self, name: &str) -> bool {
        self.protect.is_match(name)
    }
}

fn build(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        builder.add(Glob::new(p)?);
    }
    Ok(builder.build()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Rules;

    fn set() -> RuleSet {
        RuleSet::compile(&Rules::default()).unwrap()
    }

    #[test]
    fn file_tiers() {
        let s = set();
        assert_eq!(s.file_tier("main.rs"), 0);
        assert_eq!(s.file_tier("index.js"), 0);
        assert_eq!(s.file_tier("README.md"), 1);
        assert_eq!(s.file_tier("Cargo.toml"), 1);
        assert_eq!(s.file_tier("helper.py"), 2);
        // Package/module markers are NOT entrypoints.
        assert_eq!(s.file_tier("__init__.py"), 2);
        assert_eq!(s.file_tier("mod.rs"), 2);
    }

    #[test]
    fn dir_classes() {
        let s = set();
        assert_eq!(s.dir_class("src"), DirClass::Protected);
        assert_eq!(s.dir_class("tests"), DirClass::Protected);
        assert_eq!(s.dir_class("node_modules"), DirClass::Noise);
        assert_eq!(s.dir_class("target"), DirClass::Noise);
        assert_eq!(s.dir_class("random"), DirClass::Normal);
    }
}
