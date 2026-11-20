//! FEEL tokenizer.

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    /// Name-like word (identifiers and keywords).
    Name(String),
    Number(String),
    Str(String),
    /// `@"..."` literal.
    At(String),
    Punct(&'static str),
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn is_punct(&self, p: &str) -> bool {
        matches!(&self.tok, Tok::Punct(q) if *q == p)
    }

    pub fn is_name(&self, n: &str) -> bool {
        matches!(&self.tok, Tok::Name(q) if q == n)
    }

    pub fn name(&self) -> Option<&str> {
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

fn parse_string_body(chars: &[(usize, char)], i: &mut usize) -> Result<String, LexError> {
    // assumes chars[*i] is the opening quote
    *i += 1;
    let mut s = String::new();
    loop {
        if *i >= chars.len() {
            return Err(LexError("unterminated string".into()));
        }
        let c = chars[*i].1;
        *i += 1;
        match c {
            '"' => break,
            '\\' => {
                if *i >= chars.len() {
                    return Err(LexError("bad escape".into()));
                }
                let e = chars[*i].1;
                *i += 1;
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
                        let hex: String = chars[*i..].iter().take(4).map(|(_, c)| *c).collect();
                        if hex.len() != 4 {
                            return Err(LexError("bad unicode escape".into()));
                        }
                        *i += 4;
                        let code = u32::from_str_radix(&hex, 16)
                            .map_err(|_| LexError("bad unicode escape".into()))?;
                        if (0xD800..0xDC00).contains(&code) {
                            // surrogate pair
                            if *i + 6 <= chars.len()
                                && chars[*i].1 == '\\'
                                && chars[*i + 1].1 == 'u'
                            {
                                let hex2: String =
                                    chars[*i + 2..*i + 6].iter().map(|(_, c)| *c).collect();
                                let low = u32::from_str_radix(&hex2, 16)
                                    .map_err(|_| LexError("bad unicode escape".into()))?;
                                if (0xDC00..0xE000).contains(&low) {
                                    *i += 6;
                                    let cp = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                    s.push(
                                        char::from_u32(cp)
                                            .ok_or_else(|| LexError("bad code point".into()))?,
                                    );
                                    continue;
                                }
                            }
                            return Err(LexError("lone surrogate".into()));
                        }
                        s.push(
                            char::from_u32(code)
                                .ok_or_else(|| LexError("bad code point".into()))?,
                        );
                    }
                    'U' => {
                        let hex: String = chars[*i..].iter().take(6).map(|(_, c)| *c).collect();
                        if hex.len() != 6 {
                            return Err(LexError("bad unicode escape".into()));
                        }
                        *i += 6;
                        let code = u32::from_str_radix(&hex, 16)
                            .map_err(|_| LexError("bad unicode escape".into()))?;
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
            c => s.push(c),
        }
    }
    Ok(s)
}

pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let n = chars.len();
    let pos = |i: usize| if i < n { chars[i].0 } else { src.len() };
    let mut toks: Vec<Token> = Vec::new();
    let mut i = 0;
    while i < n {
        let c = chars[i].1;
        if is_whitespace(c) {
            i += 1;
            continue;
        }
        let start = pos(i);
        // comments
        if c == '/' && i + 1 < n && chars[i + 1].1 == '/' {
            while i < n && chars[i].1 != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < n && chars[i + 1].1 == '*' {
            i += 2;
            while i + 1 < n && !(chars[i].1 == '*' && chars[i + 1].1 == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        if c == '"' {
            let s = parse_string_body(&chars, &mut i)?;
            toks.push(Token {
                tok: Tok::Str(s),
                start,
                end: pos(i),
            });
            continue;
        }
        if c == '@' && i + 1 < n && chars[i + 1].1 == '"' {
            i += 1;
            let s = parse_string_body(&chars, &mut i)?;
            toks.push(Token {
                tok: Tok::At(s),
                start,
                end: pos(i),
            });
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
        let starts_number = c.is_ascii_digit()
            || (c == '.' && i + 1 < n && chars[i + 1].1.is_ascii_digit() && !prev_is_value);
        if starts_number {
            let mut s = String::new();
            while i < n && chars[i].1.is_ascii_digit() {
                s.push(chars[i].1);
                i += 1;
            }
            if i + 1 < n && chars[i].1 == '.' && chars[i + 1].1.is_ascii_digit() {
                s.push('.');
                i += 1;
                while i < n && chars[i].1.is_ascii_digit() {
                    s.push(chars[i].1);
                    i += 1;
                }
            } else if i < n && chars[i].1 == '.' && s.is_empty() {
                // leading dot handled above
            }
            if s.is_empty() && i < n && chars[i].1 == '.' {
                s.push('0');
                s.push('.');
                i += 1;
                while i < n && chars[i].1.is_ascii_digit() {
                    s.push(chars[i].1);
                    i += 1;
                }
            }
            // exponent
            if i < n && (chars[i].1 == 'e' || chars[i].1 == 'E') {
                let mut j = i + 1;
                if j < n && (chars[j].1 == '+' || chars[j].1 == '-') {
                    j += 1;
                }
                if j < n && chars[j].1.is_ascii_digit() {
                    s.push('e');
                    if chars[i + 1].1 == '-' {
                        s.push('-');
                    }
                    i = j;
                    while i < n && chars[i].1.is_ascii_digit() {
                        s.push(chars[i].1);
                        i += 1;
                    }
                }
            }
            toks.push(Token {
                tok: Tok::Number(s),
                start,
                end: pos(i),
            });
            continue;
        }
        if is_name_start(c) {
            let mut s = String::new();
            while i < n && is_name_part(chars[i].1) {
                s.push(chars[i].1);
                i += 1;
            }
            toks.push(Token {
                tok: Tok::Name(s),
                start,
                end: pos(i),
            });
            continue;
        }
        let next = if i + 1 < n { chars[i + 1].1 } else { '\0' };
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
            end: pos(i),
        });
    }
    toks.push(Token {
        tok: Tok::Eof,
        start: src.len(),
        end: src.len(),
    });
    Ok(toks)
}

/// Is `s` already normalized (ASCII, single inner spaces, no other whitespace)?
fn is_normalized(s: &str) -> bool {
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
