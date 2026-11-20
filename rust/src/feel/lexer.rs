//! FEEL tokenizer.
//!
//! Tokens borrow their text from the source; only string literals containing
//! escape sequences allocate.

use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq)]
pub enum Tok<'a> {
    /// Name-like word (identifiers and keywords).
    Name(&'a str),
    Number(&'a str),
    Str(Cow<'a, str>),
    /// `@"..."` literal.
    At(Cow<'a, str>),
    Punct(&'static str),
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token<'a> {
    pub tok: Tok<'a>,
    pub start: usize,
    pub end: usize,
}

impl<'a> Token<'a> {
    pub fn is_punct(&self, p: &str) -> bool {
        matches!(&self.tok, Tok::Punct(q) if *q == p)
    }

    pub fn is_name(&self, n: &str) -> bool {
        matches!(&self.tok, Tok::Name(q) if *q == n)
    }

    pub fn name(&self) -> Option<&'a str> {
        match &self.tok {
            Tok::Name(n) => Some(n),
            _ => None,
        }
    }
}

pub fn is_whitespace(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '\u{FEFF}' | '\u{200B}' | '\u{0085}' | '\u{00A0}' | '\u{1680}' | '\u{180E}' | '\u{2000}'
                ..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
        )
}

fn is_name_start(c: char) -> bool {
    c.is_alphabetic()
        || c == '_'
        || c == '?'
        || (!c.is_ascii() && !is_whitespace(c) && !matches!(c, '\u{2018}'..='\u{201F}'))
}

fn is_name_part(c: char) -> bool {
    is_name_start(c)
        || c.is_ascii_digit()
        || c == '\''
        || matches!(c, '\u{0300}'..='\u{036F}' | '\u{203F}'..='\u{2040}' | '\u{00B7}')
}

#[derive(Debug, Clone)]
pub struct LexError(pub String);

/// Character at byte offset `i` and its encoded length.
#[inline]
fn char_at(src: &str, i: usize) -> (char, usize) {
    let b = src.as_bytes()[i];
    if b < 0x80 {
        (b as char, 1)
    } else {
        let c = src[i..].chars().next().unwrap();
        (c, c.len_utf8())
    }
}

fn hex_digits(src: &str, i: usize, n: usize) -> Result<u32, LexError> {
    let h = src
        .get(i..i + n)
        .ok_or_else(|| LexError("bad unicode escape".into()))?;
    if !h.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(LexError("bad unicode escape".into()));
    }
    u32::from_str_radix(h, 16).map_err(|_| LexError("bad unicode escape".into()))
}

/// Parse a string literal whose opening quote is at byte `q`; returns the
/// content and the offset after the closing quote.
fn parse_string(src: &str, q: usize) -> Result<(Cow<'_, str>, usize), LexError> {
    let b = src.as_bytes();
    let start = q + 1;
    let mut i = start;
    // fast path: no escapes
    while i < b.len() && b[i] != b'"' && b[i] != b'\\' {
        i += 1;
    }
    if i >= b.len() {
        return Err(LexError("unterminated string".into()));
    }
    if b[i] == b'"' {
        return Ok((Cow::Borrowed(&src[start..i]), i + 1));
    }
    let mut s = String::from(&src[start..i]);
    loop {
        if i >= b.len() {
            return Err(LexError("unterminated string".into()));
        }
        match b[i] {
            b'"' => return Ok((Cow::Owned(s), i + 1)),
            b'\\' => {
                i += 1;
                if i >= b.len() {
                    return Err(LexError("bad escape".into()));
                }
                let (e, elen) = char_at(src, i);
                i += elen;
                match e {
                    'n' => s.push('\n'),
                    'r' => s.push('\r'),
                    't' => s.push('\t'),
                    'b' => s.push('\u{8}'),
                    'f' => s.push('\u{c}'),
                    '"' => s.push('"'),
                    '\'' => s.push('\''),
                    '\\' => s.push('\\'),
                    'u' => {
                        let code = hex_digits(src, i, 4)?;
                        i += 4;
                        if (0xD800..0xDC00).contains(&code) {
                            // surrogate pair
                            if src.get(i..i + 2) == Some("\\u")
                                && let Ok(low) = hex_digits(src, i + 2, 4)
                                && (0xDC00..0xE000).contains(&low)
                            {
                                i += 6;
                                let cp = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                s.push(
                                    char::from_u32(cp)
                                        .ok_or_else(|| LexError("bad code point".into()))?,
                                );
                                continue;
                            }
                            return Err(LexError("lone surrogate".into()));
                        }
                        s.push(
                            char::from_u32(code)
                                .ok_or_else(|| LexError("bad code point".into()))?,
                        );
                    }
                    'U' => {
                        let code = hex_digits(src, i, 6)?;
                        i += 6;
                        s.push(
                            char::from_u32(code)
                                .ok_or_else(|| LexError("bad code point".into()))?,
                        );
                    }
                    other => {
                        // be lenient: keep unknown escapes verbatim
                        s.push('\\');
                        s.push(other);
                    }
                }
            }
            _ => {
                let (c, clen) = char_at(src, i);
                s.push(c);
                i += clen;
            }
        }
    }
}

pub fn tokenize(src: &str) -> Result<Vec<Token<'_>>, LexError> {
    let b = src.as_bytes();
    let n = b.len();
    let mut toks: Vec<Token> = Vec::with_capacity(n / 4 + 2);
    let mut i = 0;
    while i < n {
        let (c, clen) = char_at(src, i);
        if is_whitespace(c) {
            i += clen;
            continue;
        }
        let start = i;
        let next = b.get(i + 1).copied().unwrap_or(0);
        // comments
        if c == '/' && next == b'/' {
            while i < n && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && next == b'*' {
            i = match src[i + 2..].find("*/") {
                Some(p) => i + 2 + p + 2,
                None => n,
            };
            continue;
        }
        if c == '"' {
            let (s, end) = parse_string(src, i)?;
            toks.push(Token {
                tok: Tok::Str(s),
                start,
                end,
            });
            i = end;
            continue;
        }
        if c == '@' && next == b'"' {
            let (s, end) = parse_string(src, i + 1)?;
            toks.push(Token {
                tok: Tok::At(s),
                start,
                end,
            });
            i = end;
            continue;
        }
        let prev_is_value = matches!(
            toks.last().map(|t| &t.tok),
            Some(Tok::Name(_))
                | Some(Tok::Number(_))
                | Some(Tok::Str(_))
                | Some(Tok::At(_))
                | Some(Tok::Punct(")"))
                | Some(Tok::Punct("]"))
                | Some(Tok::Punct("}"))
        );
        let starts_number =
            c.is_ascii_digit() || (c == '.' && next.is_ascii_digit() && !prev_is_value);
        if starts_number {
            let mut j = i;
            while j < n && b[j].is_ascii_digit() {
                j += 1;
            }
            if j + 1 < n && b[j] == b'.' && b[j + 1].is_ascii_digit() {
                j += 1;
                while j < n && b[j].is_ascii_digit() {
                    j += 1;
                }
            }
            // exponent
            if j < n && (b[j] == b'e' || b[j] == b'E') {
                let mut k = j + 1;
                if k < n && (b[k] == b'+' || b[k] == b'-') {
                    k += 1;
                }
                if k < n && b[k].is_ascii_digit() {
                    while k < n && b[k].is_ascii_digit() {
                        k += 1;
                    }
                    j = k;
                }
            }
            toks.push(Token {
                tok: Tok::Number(&src[i..j]),
                start,
                end: j,
            });
            i = j;
            continue;
        }
        if is_name_start(c) {
            let mut j = i + clen;
            while j < n {
                let (d, dlen) = char_at(src, j);
                if !is_name_part(d) {
                    break;
                }
                j += dlen;
            }
            toks.push(Token {
                tok: Tok::Name(&src[i..j]),
                start,
                end: j,
            });
            i = j;
            continue;
        }
        let next = next as char;
        let (p, len): (&'static str, usize) = match (c, next) {
            ('.', '.') => ("..", 2),
            ('*', '*') => ("**", 2),
            ('!', '=') => ("!=", 2),
            ('<', '=') => ("<=", 2),
            ('>', '=') => (">=", 2),
            ('-', '>') => ("->", 2),
            ('(', _) => ("(", 1),
            (')', _) => (")", 1),
            ('[', _) => ("[", 1),
            (']', _) => ("]", 1),
            ('{', _) => ("{", 1),
            ('}', _) => ("}", 1),
            (',', _) => (",", 1),
            (':', _) => (":", 1),
            ('.', _) => (".", 1),
            ('+', _) => ("+", 1),
            ('-', _) => ("-", 1),
            ('*', _) => ("*", 1),
            ('/', _) => ("/", 1),
            ('=', _) => ("=", 1),
            ('<', _) => ("<", 1),
            ('>', _) => (">", 1),
            _ => return Err(LexError(format!("unexpected character '{}'", c))),
        };
        i += len;
        toks.push(Token {
            tok: Tok::Punct(p),
            start,
            end: i,
        });
    }
    toks.push(Token {
        tok: Tok::Eof,
        start: n,
        end: n,
    });
    Ok(toks)
}

/// Is `s` already normalized (ASCII, single inner spaces, no other whitespace)?
pub fn is_normalized(s: &str) -> bool {
    let b = s.as_bytes();
    if b.is_empty() || b[0] == b' ' || b[b.len() - 1] == b' ' {
        return b.is_empty();
    }
    let mut prev_space = false;
    for &c in b {
        if c >= 0x80 || matches!(c, b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
            return false;
        }
        let space = c == b' ';
        if space && prev_space {
            return false;
        }
        prev_space = space;
    }
    true
}

/// Leading word of a name as the lexer would tokenize it.
pub fn first_word(name: &str) -> &str {
    let end = name
        .char_indices()
        .find(|(i, c)| {
            if *i == 0 {
                !is_name_start(*c)
            } else {
                !is_name_part(*c)
            }
        })
        .map(|(i, _)| i)
        .unwrap_or(name.len());
    &name[..end]
}

/// Collapse whitespace runs and trim.
pub fn normalize_name(s: &str) -> String {
    if is_normalized(s) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut ws = false;
    for c in s.chars() {
        if is_whitespace(c) {
            ws = true;
        } else {
            if ws && !out.is_empty() {
                out.push(' ');
            }
            ws = false;
            out.push(c);
        }
    }
    out
}
