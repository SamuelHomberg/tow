use std::cmp::Ordering;

use crate::cli::{Config, SortKey};
use crate::model::Node;

/// Sort a slice of nodes according to the configured sort key and direction.
pub fn sort_nodes(nodes: &mut [Node], cfg: &Config) {
    match cfg.sort_key {
        SortKey::None => {}
        SortKey::Name => nodes.sort_by(|a, b| nat_cmp(a.name(), b.name())),
        SortKey::Mtime => nodes.sort_by(|a, b| {
            b.mtime()
                .cmp(&a.mtime())
                .then_with(|| nat_cmp(a.name(), b.name()))
        }),
        SortKey::Commits => nodes.sort_by(|a, b| {
            b.change_time()
                .cmp(&a.change_time())
                .then_with(|| nat_cmp(a.name(), b.name()))
        }),
        SortKey::Size => nodes.sort_by(|a, b| {
            b.size()
                .cmp(&a.size())
                .then_with(|| nat_cmp(a.name(), b.name()))
        }),
        SortKey::Version => nodes.sort_by(|a, b| nat_cmp(a.name(), b.name())),
    }
    if cfg.reverse {
        nodes.reverse();
    }
}

/// Natural (version-aware) comparison: digit runs compare numerically.
pub fn nat_cmp(a: &str, b: &str) -> Ordering {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let (mut i, mut j) = (0usize, 0usize);
    while i < a.len() && j < b.len() {
        let (ca, cb) = (a[i], b[j]);
        if ca.is_ascii_digit() && cb.is_ascii_digit() {
            let mut i2 = i;
            let mut j2 = j;
            while i2 < a.len() && a[i2].is_ascii_digit() {
                i2 += 1;
            }
            while j2 < b.len() && b[j2].is_ascii_digit() {
                j2 += 1;
            }
            // Skip leading zeros.
            let (mut ai, mut bi) = (i, j);
            while ai < i2 && a[ai] == b'0' {
                ai += 1;
            }
            while bi < j2 && b[bi] == b'0' {
                bi += 1;
            }
            let na = &a[ai..i2];
            let nb = &b[bi..j2];
            let ord = match na.len().cmp(&nb.len()) {
                Ordering::Equal => na.cmp(nb),
                o => o,
            };
            if ord != Ordering::Equal {
                return ord;
            }
            i = i2;
            j = j2;
        } else {
            let ord = ca.to_ascii_lowercase().cmp(&cb.to_ascii_lowercase());
            if ord != Ordering::Equal {
                return ord;
            }
            i += 1;
            j += 1;
        }
    }
    a.len().cmp(&b.len())
}

#[cfg(test)]
mod tests {
    use super::nat_cmp;
    use std::cmp::Ordering;

    #[test]
    fn natural_order() {
        let mut v = vec!["file10", "file2", "file1", "file20"];
        v.sort_by(|a, b| nat_cmp(a, b));
        assert_eq!(v, vec!["file1", "file2", "file10", "file20"]);
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(nat_cmp("Alpha", "beta"), Ordering::Less);
    }
}
