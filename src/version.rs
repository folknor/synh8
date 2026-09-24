//! Debian version comparison (the dpkg algorithm), in pure Rust so that
//! sorting tens of thousands of packages by version needs no FFI calls.

use std::cmp::Ordering;

/// Compare two Debian version strings (`[epoch:]upstream[-revision]`).
pub fn compare(a: &str, b: &str) -> Ordering {
    let (ea, ua, ra) = split(a);
    let (eb, ub, rb) = split(b);
    ea.cmp(&eb)
        .then_with(|| verrevcmp(ua.as_bytes(), ub.as_bytes()))
        .then_with(|| verrevcmp(ra.as_bytes(), rb.as_bytes()))
}

/// Split into (epoch, upstream, revision). A missing epoch is 0, a missing
/// revision is empty.
fn split(v: &str) -> (u64, &str, &str) {
    let (epoch, rest) = match v.split_once(':') {
        Some((e, rest)) if e.bytes().all(|c| c.is_ascii_digit()) => (e.parse().unwrap_or(0), rest),
        _ => (0, v),
    };
    match rest.rsplit_once('-') {
        Some((upstream, revision)) => (epoch, upstream, revision),
        None => (epoch, rest, ""),
    }
}

/// Sort weight of a non-digit character: `~` sorts before everything,
/// including the end of the string; letters sort before other symbols.
fn order(c: Option<u8>) -> i32 {
    match c {
        None => 0,
        Some(c) if c.is_ascii_digit() => 0,
        Some(c) if c.is_ascii_alphabetic() => i32::from(c),
        Some(b'~') => -1,
        Some(c) => i32::from(c) + 256,
    }
}

fn is_digit(c: Option<&u8>) -> bool {
    c.is_some_and(u8::is_ascii_digit)
}

fn verrevcmp(a: &[u8], b: &[u8]) -> Ordering {
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        // Non-digit prefix, compared character by character
        while (i < a.len() && !a[i].is_ascii_digit()) || (j < b.len() && !b[j].is_ascii_digit()) {
            let ac = order(a.get(i).copied());
            let bc = order(b.get(j).copied());
            if ac != bc {
                return ac.cmp(&bc);
            }
            i += 1;
            j += 1;
        }
        // Digit run, compared numerically
        while a.get(i) == Some(&b'0') {
            i += 1;
        }
        while b.get(j) == Some(&b'0') {
            j += 1;
        }
        let mut first_diff = Ordering::Equal;
        while is_digit(a.get(i)) && is_digit(b.get(j)) {
            if first_diff == Ordering::Equal {
                first_diff = a[i].cmp(&b[j]);
            }
            i += 1;
            j += 1;
        }
        if is_digit(a.get(i)) {
            return Ordering::Greater;
        }
        if is_digit(b.get(j)) {
            return Ordering::Less;
        }
        if first_diff != Ordering::Equal {
            return first_diff;
        }
    }
    Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;
    use Ordering::{Equal, Greater, Less};

    #[test]
    fn numeric_not_lexicographic() {
        assert_eq!(compare("10.0", "9.0"), Greater);
        assert_eq!(compare("1.0.10", "1.0.9"), Greater);
    }

    #[test]
    fn tilde_sorts_before_release() {
        assert_eq!(compare("1.0~rc1", "1.0"), Less);
        assert_eq!(compare("1.0~rc1", "1.0~rc2"), Less);
        assert_eq!(compare("1.0~", "1.0~~"), Greater);
    }

    #[test]
    fn epochs_and_revisions() {
        assert_eq!(compare("2:1.0", "1:9.9"), Greater);
        assert_eq!(compare("1.0", "0:1.0"), Equal);
        assert_eq!(compare("1.0-1", "1.0-2"), Less);
        assert_eq!(compare("1.0-1ubuntu1", "1.0-1"), Greater);
        assert_eq!(compare("1.2-3-4", "1.2-3-5"), Less);
    }

    #[test]
    fn letters_and_symbols() {
        assert_eq!(compare("1.0a", "1.0"), Greater);
        assert_eq!(compare("1.0+b1", "1.0"), Greater);
        assert_eq!(compare("1.0a", "1.0+"), Less);
        assert_eq!(compare("1.00", "1.0"), Equal);
    }
}
