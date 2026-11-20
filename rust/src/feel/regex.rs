//! Translation of XML Schema / XPath regular expressions to `fancy_regex`.

use fancy_regex::{Regex, RegexBuilder};

/// Compile an XPath regular expression (with flags), caching compiled expressions.
pub fn translate_regex(pattern: &str, flags: &str) -> Option<Regex> {
    use std::cell::RefCell;
    thread_local! {
        static CACHE: RefCell<rustc_hash::FxHashMap<(String, String), Option<Regex>>> =
            RefCell::new(rustc_hash::FxHashMap::default());
    }
    CACHE.with(|c| {
        if let Some(r) = c.borrow().get(&(pattern.to_string(), flags.to_string())) {
            return r.clone();
        }
        let r = compile_regex(pattern, flags);
        let mut c = c.borrow_mut();
        if c.len() >= 512 {
            c.clear();
        }
        c.insert((pattern.to_string(), flags.to_string()), r.clone());
        r
    })
}

fn compile_regex(pattern: &str, flags: &str) -> Option<Regex> {
    let mut case_insensitive = false;
    let mut dot_all = false;
    let mut multi_line = false;
    let mut extended = false;
    let mut literal = false;
    for f in flags.chars() {
        match f {
            'i' => case_insensitive = true,
            's' => dot_all = true,
            'm' => multi_line = true,
            'x' => extended = true,
            'q' => literal = true,
            _ => return None,
        }
    }
    let body = if literal {
        fancy_regex::escape(pattern).to_string()
    } else {
        translate_body(pattern, extended, dot_all)?
    };
    let mut prefix = String::new();
    if case_insensitive || dot_all || multi_line {
        prefix.push_str("(?");
        if case_insensitive {
            prefix.push('i');
        }
        if dot_all {
            prefix.push('s');
        }
        if multi_line {
            prefix.push('m');
        }
        prefix.push(')');
    }
    let full = format!("{}{}", prefix, body);
    RegexBuilder::new(&full).build().ok()
}

/// Remove whitespace outside character classes (XPath `x` flag).
fn strip_whitespace(p: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    let mut escaped = false;
    for c in p.chars() {
        if escaped {
            if c.is_whitespace() {
                continue;
            }
            escaped = false;
            out.push(c);
            continue;
        }
        match c {
            '\\' => {
                escaped = true;
                out.push(c);
            }
            '[' => {
                depth += 1;
                out.push(c);
            }
            ']' => {
                if depth > 0 {
                    depth -= 1;
                }
                out.push(c);
            }
            c if c.is_whitespace() && depth == 0 => {}
            c => out.push(c),
        }
    }
    // whitespace following a backslash outside classes is also removed: `\ s` -> `\s`
    let mut res = String::new();
    let chars: Vec<char> = out.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        res.push(chars[i]);
        i += 1;
    }
    res
}

fn unicode_block(name: &str) -> Option<&'static str> {
    Some(match name {
        "BasicLatin" => "\\x{0000}-\\x{007F}",
        "Latin-1Supplement" => "\\x{0080}-\\x{00FF}",
        "LatinExtended-A" => "\\x{0100}-\\x{017F}",
        "LatinExtended-B" => "\\x{0180}-\\x{024F}",
        "IPAExtensions" => "\\x{0250}-\\x{02AF}",
        "SpacingModifierLetters" => "\\x{02B0}-\\x{02FF}",
        "CombiningDiacriticalMarks" => "\\x{0300}-\\x{036F}",
        "Greek" | "GreekandCoptic" => "\\x{0370}-\\x{03FF}",
        "Cyrillic" => "\\x{0400}-\\x{04FF}",
        "Armenian" => "\\x{0530}-\\x{058F}",
        "Hebrew" => "\\x{0590}-\\x{05FF}",
        "Arabic" => "\\x{0600}-\\x{06FF}",
        "Devanagari" => "\\x{0900}-\\x{097F}",
        "Thai" => "\\x{0E00}-\\x{0E7F}",
        "LatinExtendedAdditional" => "\\x{1E00}-\\x{1EFF}",
        "GreekExtended" => "\\x{1F00}-\\x{1FFF}",
        "GeneralPunctuation" => "\\x{2000}-\\x{206F}",
        "CurrencySymbols" => "\\x{20A0}-\\x{20CF}",
        "LetterlikeSymbols" => "\\x{2100}-\\x{214F}",
        "NumberForms" => "\\x{2150}-\\x{218F}",
        "Arrows" => "\\x{2190}-\\x{21FF}",
        "MathematicalOperators" => "\\x{2200}-\\x{22FF}",
        "BoxDrawing" => "\\x{2500}-\\x{257F}",
        "CJKSymbolsandPunctuation" => "\\x{3000}-\\x{303F}",
        "Hiragana" => "\\x{3040}-\\x{309F}",
        "Katakana" => "\\x{30A0}-\\x{30FF}",
        "CJKUnifiedIdeographs" => "\\x{4E00}-\\x{9FFF}",
        "HangulSyllables" => "\\x{AC00}-\\x{D7AF}",
        "PrivateUseArea" | "PrivateUse" => "\\x{E000}-\\x{F8FF}",
        "AlphabeticPresentationForms" => "\\x{FB00}-\\x{FB4F}",
        "HalfwidthandFullwidthForms" => "\\x{FF00}-\\x{FFEF}",
        "Specials" => "\\x{FFF0}-\\x{FFFF}",
        _ => return None,
    })
}

fn translate_body(p: &str, extended: bool, dot_all: bool) -> Option<String> {
    let stripped;
    let p = if extended {
        stripped = strip_whitespace(p);
        stripped.as_str()
    } else {
        p
    };
    let chars: Vec<char> = p.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut class_depth = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '.' if class_depth == 0 && !dot_all => {
                out.push_str("[^\\n\\r]");
            }
            '\\' => {
                let n = *chars.get(i + 1)?;
                match n {
                    'i' => out.push_str(if class_depth > 0 {
                        "_:A-Za-z"
                    } else {
                        "[_:A-Za-z]"
                    }),
                    'I' => out.push_str("[^_:A-Za-z]"),
                    'c' => out.push_str(if class_depth > 0 {
                        "-._:A-Za-z0-9"
                    } else {
                        "[-._:A-Za-z0-9]"
                    }),
                    'C' => out.push_str("[^-._:A-Za-z0-9]"),
                    'p' | 'P' => {
                        let rest: String = chars[i + 2..].iter().collect();
                        if let Some(block) = rest.strip_prefix("{Is") {
                            let end = block.find('}')?;
                            let range = unicode_block(&block[..end])?;
                            let neg = n == 'P';
                            if class_depth > 0 {
                                if neg {
                                    return None;
                                }
                                out.push_str(range);
                            } else {
                                out.push_str(if neg { "[^" } else { "[" });
                                out.push_str(range);
                                out.push(']');
                            }
                            i += 2 + 3 + end + 1;
                            continue;
                        }
                        out.push('\\');
                        out.push(n);
                    }
                    _ => {
                        out.push('\\');
                        out.push(n);
                    }
                }
                i += 2;
                continue;
            }
            '[' => {
                class_depth += 1;
                out.push('[');
                if chars.get(i + 1) == Some(&'^') {
                    out.push('^');
                    i += 1;
                }
            }
            ']' => {
                if class_depth > 0 {
                    class_depth -= 1;
                }
                out.push(']');
            }
            '-' if class_depth > 0 && chars.get(i + 1) == Some(&'[') => {
                // character class subtraction
                out.push_str("--");
            }
            '&' | '~' | '|' if class_depth > 0 => {
                // escape characters with special meaning in regex crate classes
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
        i += 1;
    }
    Some(out)
}

/// Translate an XPath replacement string (`$1`, `\$`, `\\`) to regex crate syntax.
pub fn translate_replacement(r: &str) -> Option<String> {
    let chars: Vec<char> = r.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' => {
                let n = *chars.get(i + 1)?;
                match n {
                    '\\' => out.push('\\'),
                    '$' => out.push_str("$$"),
                    _ => return None,
                }
                i += 2;
            }
            '$' => {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                if j == i + 1 {
                    return None;
                }
                let digits: String = chars[i + 1..j].iter().collect();
                out.push_str("${");
                out.push_str(&digits);
                out.push('}');
                i = j;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    Some(out)
}
