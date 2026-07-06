//! Minimal deterministic S-expression builder for KiCad 6+ file formats.

use std::fmt;

/// A single S-expression node.
#[derive(Clone, Debug, PartialEq)]
pub enum Sexpr {
    /// A parenthesised list of child nodes.
    List(Vec<Sexpr>),
    /// An atom printed exactly as stored (caller is responsible for quoting).
    Atom(String),
    /// A raw fragment printed exactly as stored, bypassing normal formatting.
    Raw(String),
}

impl Sexpr {
    /// Construct a list from an iterator of child nodes.
    pub fn list(children: impl IntoIterator<Item = Sexpr>) -> Self {
        Self::List(children.into_iter().collect())
    }

    /// Construct an atom.
    pub fn atom(s: impl Into<String>) -> Self {
        Self::Atom(s.into())
    }

    /// Construct a quoted string atom: `"s"`, escaping `"` and `\`.
    pub fn str(s: impl AsRef<str>) -> Self {
        Self::Atom(format!("\"{}\"", escape_str(s.as_ref())))
    }

    /// Construct a raw node.
    pub fn raw(s: impl Into<String>) -> Self {
        Self::Raw(s.into())
    }

    /// True if this node is an atom or raw text (i.e. has no children).
    fn is_leaf(&self) -> bool {
        matches!(self, Self::Atom(_) | Self::Raw(_))
    }

    fn write(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        match self {
            Self::Raw(s) => write!(f, "{}", s),
            Self::Atom(s) => write!(f, "{}", s),
            Self::List(children) => {
                if children.is_empty() {
                    return write!(f, "()");
                }
                // If every child is a leaf, keep the list on one line.
                if children.iter().all(|c| c.is_leaf()) {
                    write!(f, "(")?;
                    for (i, c) in children.iter().enumerate() {
                        if i > 0 {
                            write!(f, " ")?;
                        }
                        c.write(f, indent)?;
                    }
                    return write!(f, ")");
                }

                // Print leading leaf children on the opening line, then break
                // for nested lists. This matches KiCad's typical formatting.
                let (leaves, rest): (Vec<_>, Vec<_>) = children.iter().partition(|c| c.is_leaf());

                write!(f, "(")?;
                for (i, c) in leaves.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    c.write(f, indent)?;
                }

                if !rest.is_empty() {
                    for c in &rest {
                        writeln!(f)?;
                        for _ in 0..indent + 2 {
                            write!(f, " ")?;
                        }
                        c.write(f, indent + 2)?;
                    }
                }
                write!(f, ")")
            }
        }
    }
}

impl fmt::Display for Sexpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write(f, 0)
    }
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Convenience: `(key "val")`.
pub fn kv(key: impl AsRef<str>, val: impl AsRef<str>) -> Sexpr {
    Sexpr::list([Sexpr::atom(key.as_ref().to_string()), Sexpr::str(val)])
}

fn fnv1a_64(seed: &str, salt: u64) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET ^ salt;
    for b in seed.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Deterministic UUID-formatted string (8-4-4-4-12 hex) derived from `seed`.
pub fn deterministic_uuid(seed: &str) -> String {
    let h1 = fnv1a_64(seed, 0);
    let h2 = fnv1a_64(seed, 0x6c14_4f3a_7af5_c5d2); // arbitrary fixed salt
    let b1 = h1.to_be_bytes();
    let b2 = h2.to_be_bytes();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b1[0],
        b1[1],
        b1[2],
        b1[3],
        b1[4],
        b1[5],
        b1[6],
        b1[7],
        b2[0],
        b2[1],
        b2[2],
        b2[3],
        b2[4],
        b2[5],
        b2[6],
        b2[7]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_print_nested_list() {
        let s = Sexpr::list([
            Sexpr::atom("export"),
            Sexpr::list([Sexpr::atom("version"), Sexpr::str("E")]),
            Sexpr::list([
                Sexpr::atom("design"),
                Sexpr::list([Sexpr::atom("source"), Sexpr::str("copperleaf")]),
            ]),
        ]);
        let out = s.to_string();
        assert!(out.starts_with("(export\n"));
        assert!(out.contains("  (version \"E\")"));
        assert!(out.contains("  (design\n    (source \"copperleaf\")"));
        assert!(out.ends_with(")"));
    }

    #[test]
    fn deterministic_uuid_stable() {
        let a = deterministic_uuid("sch:U1");
        let b = deterministic_uuid("sch:U1");
        assert_eq!(a, b);
        assert_eq!(a.len(), 36);
        assert_eq!(a.matches('-').count(), 4);
    }

    #[test]
    fn deterministic_uuid_distinct() {
        let a = deterministic_uuid("sch:U1");
        let b = deterministic_uuid("sch:U2");
        assert_ne!(a, b);
    }
}
