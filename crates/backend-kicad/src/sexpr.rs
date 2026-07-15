//! Minimal deterministic S-expression builder for KiCad 10+ file formats.

use std::fmt;

use thiserror::Error;

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

/// Error returned when an S-expression cannot be parsed or read.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ParseError {
    /// Input ended while a string or list was still open.
    #[error("unexpected end of input")]
    UnexpectedEof,
    /// A closing `)` had no matching opening `(`.
    #[error("unmatched ')' at position {pos}")]
    UnmatchedParen { pos: usize },
    /// An invalid escape sequence was encountered inside a quoted string.
    #[error("bad escape sequence at position {pos}")]
    BadEscape { pos: usize },
    /// An unexpected character was found at the given byte position.
    #[error("unexpected character {ch:?} at position {pos}")]
    UnexpectedChar { ch: char, pos: usize },
    /// An I/O error occurred while reading a file.
    #[error("I/O error: {0}")]
    Io(String),
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        ParseError::Io(err.to_string())
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    LParen,
    RParen,
    Atom(String),
    String(String),
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
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

impl<'a> Parser<'a> {
    fn parse_expr(&mut self) -> Result<Sexpr, ParseError> {
        match self.tokens.get(self.pos) {
            Some(Token::LParen) => self.parse_list(),
            Some(Token::Atom(s)) => {
                self.pos += 1;
                Ok(Sexpr::Atom(s.clone()))
            }
            Some(Token::String(s)) => {
                self.pos += 1;
                Ok(Sexpr::str(s.clone()))
            }
            Some(Token::RParen) => Err(ParseError::UnmatchedParen { pos: self.pos }),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    fn parse_list(&mut self) -> Result<Sexpr, ParseError> {
        self.pos += 1;
        let mut children = Vec::new();
        loop {
            match self.tokens.get(self.pos) {
                Some(Token::RParen) => {
                    self.pos += 1;
                    return Ok(Sexpr::List(children));
                }
                Some(_) => children.push(self.parse_expr()?),
                None => return Err(ParseError::UnexpectedEof),
            }
        }
    }
}

/// Deterministic UUID-formatted string (8-4-4-4-12 hex) derived from `seed`.
pub fn deterministic_uuid(seed: &str) -> String {
    let h1 = fnv1a_64(seed, 0);
    let h2 = fnv1a_64(seed, 0x6c14_4f3a_7af5_c5d2);
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

/// Convenience: `(key "val")`.
pub fn kv(key: impl AsRef<str>, val: impl AsRef<str>) -> Sexpr {
    Sexpr::list([Sexpr::atom(key.as_ref().to_string()), Sexpr::str(val)])
}

/// Parse a KiCad S-expression string into an [`Sexpr`] tree.
pub fn parse(input: &str) -> Result<Sexpr, ParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser {
        tokens: &tokens,
        pos: 0,
    };
    let expr = parser.parse_expr()?;
    if parser.pos < tokens.len() {
        return Err(ParseError::UnmatchedParen { pos: parser.pos });
    }
    Ok(expr)
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
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

/// Tokenize a KiCad S-expression string.
fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();

    while let Some((_start, ch)) = chars.next() {
        match ch {
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            '"' => {
                let mut s = String::new();
                loop {
                    match chars.next() {
                        Some((_, '\\')) => match chars.next() {
                            Some((_, '\\')) => s.push('\\'),
                            Some((_, '"')) => s.push('"'),
                            Some((pos, _)) => return Err(ParseError::BadEscape { pos }),
                            None => return Err(ParseError::UnexpectedEof),
                        },
                        Some((_, '"')) => break,
                        Some((_, c)) => s.push(c),
                        None => return Err(ParseError::UnexpectedEof),
                    }
                }
                tokens.push(Token::String(s));
            }
            '#' => {
                for (_, c) in &mut chars {
                    if c == '\n' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {}
            _ => {
                let mut s = String::new();
                s.push(ch);
                while let Some(&(_, c)) = chars.peek() {
                    if c.is_whitespace() || c == '(' || c == ')' || c == '"' || c == '#' {
                        break;
                    }
                    s.push(c);
                    chars.next();
                }
                tokens.push(Token::Atom(s));
            }
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_uuid_stable() {
        let a = deterministic_uuid("sch:U1");
        let b = deterministic_uuid("sch:U1");
        assert_eq!(a, b);
        assert_eq!(a.len(), 36);
    }

    #[test]
    fn parse_simple_list() {
        let s = parse("(foo bar)").unwrap();
        assert_eq!(s, Sexpr::list([Sexpr::atom("foo"), Sexpr::atom("bar")]));
    }

    #[test]
    fn parse_quoted_string() {
        let s = parse("(name \"VDD\")").unwrap();
        assert_eq!(s, Sexpr::list([Sexpr::atom("name"), Sexpr::str("VDD")]));
    }
}
