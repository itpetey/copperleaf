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

    /// Return the unquoted string value of a quoted atom, or the raw atom
    /// text for unquoted atoms. Returns an empty string for lists.
    pub fn as_string(&self) -> String {
        match self {
            Self::Atom(s) => {
                if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
                    s[1..s.len() - 1]
                        .replace("\\\"", "\"")
                        .replace("\\\\", "\\")
                } else {
                    s.clone()
                }
            }
            _ => String::new(),
        }
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

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        ParseError::Io(err.to_string())
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
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
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
                            Some((_, 'n')) => s.push('\n'),
                            Some((_, 't')) => s.push('\t'),
                            Some((_, 'r')) => s.push('\r'),
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
