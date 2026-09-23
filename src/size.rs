use crate::model::{DirNode, Node};

/// Format a byte count in a human-readable way.
///
/// `si == true` uses powers of 1000, otherwise powers of 1024. Values below
/// 1 KiB are printed as plain integers (matching `tree -h` behavior).
pub fn human(bytes: u64, si: bool) -> String {
    if bytes < 1000 {
        return bytes.to_string();
    }
    let base = if si { 1000.0f64 } else { 1024.0f64 };
    let units = ["K", "M", "G", "T", "P", "E"];
    let mut value = bytes as f64;
    let mut idx = 0usize;
    while value >= base && idx < units.len() - 1 {
        value /= base;
        idx += 1;
    }
    if idx == 0 {
        return bytes.to_string();
    }
    format!("{value:.1}{}", units[idx - 1])
}

/// Recursively accumulate directory sizes (for `--du`). Returns the size of
/// `root`, updating every `DirNode` size along the way.
pub fn accumulate(root: &mut DirNode) -> u64 {
    let mut total = 0u64;
    for child in &mut root.children {
        total += match child {
            Node::File(f) => f.size,
            Node::Dir(d) => accumulate(d),
        };
    }
    root.size = total;
    total
}

#[cfg(test)]
mod tests {
    use super::human;

    #[test]
    fn human_sizes() {
        assert_eq!(human(0, false), "0");
        assert_eq!(human(512, false), "512");
        assert_eq!(human(999, false), "999");
        assert_eq!(human(1024, false), "1.0K");
        assert_eq!(human(1536, false), "1.5K");
        assert_eq!(human(1024 * 1024, false), "1.0M");
        assert_eq!(human(1000, true), "1.0K");
        assert_eq!(human(1500, true), "1.5K");
    }
}
