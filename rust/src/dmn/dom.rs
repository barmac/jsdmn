//! Minimal, fast XML reader producing a lightweight element tree.
//!
//! It supports what DMN files use: elements, attributes, character data,
//! CDATA sections, comments, processing instructions, a DOCTYPE and the
//! predefined / numeric entities. Element and attribute names are reduced to
//! their local part (namespace prefixes are dropped). Whitespace-only text is
//! ignored, and subtrees of elements listed in `skip` (e.g. diagram
//! interchange) are skipped without building nodes.

use memchr::{memchr, memchr2, memchr3, memmem};
use std::borrow::Cow;

/// An element borrowing names and (entity free) values from the source text.
#[derive(Debug, Default, Clone)]
pub struct Element<'a> {
    pub name: &'a str,
    pub attrs: Vec<(&'a str, Cow<'a, str>)>,
    pub text: Cow<'a, str>,
    pub children: Vec<Element<'a>>,
}

impl Element<'_> {
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| &**v)
    }
}

/// Decode `raw`, borrowing it when no decoding is necessary.
fn decode_cow(raw: &str, attr: bool) -> Result<Cow<'_, str>, String> {
    if !needs_decoding(raw, attr) {
        return Ok(Cow::Borrowed(raw));
    }
    let mut out = String::with_capacity(raw.len());
    decode(raw, attr, &mut out)?;
    Ok(Cow::Owned(out))
}

/// Append decoded text to an element's text.
fn append_text<'a>(text: &mut Cow<'a, str>, raw: &'a str, verbatim: bool) -> Result<(), String> {
    if text.is_empty() {
        *text = if verbatim {
            Cow::Borrowed(raw)
        } else {
            decode_cow(raw, false)?
        };
        return Ok(());
    }
    let t = text.to_mut();
    if verbatim {
        t.push_str(raw);
    } else {
        decode(raw, false, t)?;
    }
    Ok(())
}

struct Parser<'a> {
    src: &'a str,
    b: &'a [u8],
    pos: usize,
}

#[inline]
fn local(name: &str) -> &str {
    match name.as_bytes().iter().rposition(|&c| c == b':') {
        Some(i) => &name[i + 1..],
        None => name,
    }
}

/// Does `raw` contain characters that require decoding?
#[inline]
fn needs_decoding(raw: &str, attr: bool) -> bool {
    let b = raw.as_bytes();
    if attr {
        memchr3(b'&', b'\r', b'\n', b).is_some() || memchr(b'\t', b).is_some()
    } else {
        memchr2(b'&', b'\r', b).is_some()
    }
}

fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r')
}

fn is_name_end(c: u8) -> bool {
    is_ws(c) || c == b'>' || c == b'/' || c == b'='
}

/// Decode entities and normalize line ends; `attr` also normalizes whitespace.
fn decode(raw: &str, attr: bool, out: &mut String) -> Result<(), String> {
    let bytes = raw.as_bytes();
    if !needs_decoding(raw, attr) {
        out.push_str(raw);
        return Ok(());
    }
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'&' => {
                let end = memchr(b';', &bytes[i..]).ok_or("unterminated entity")? + i;
                let ent = &raw[i + 1..end];
                match ent {
                    "lt" => out.push('<'),
                    "gt" => out.push('>'),
                    "amp" => out.push('&'),
                    "quot" => out.push('"'),
                    "apos" => out.push('\''),
                    _ => {
                        let code = if let Some(h) = ent.strip_prefix("#x") {
                            u32::from_str_radix(h, 16).ok()
                        } else if let Some(d) = ent.strip_prefix('#') {
                            d.parse::<u32>().ok()
                        } else {
                            None
                        };
                        let ch = code
                            .and_then(char::from_u32)
                            .ok_or_else(|| format!("unknown entity &{};", ent))?;
                        out.push(ch);
                    }
                }
                i = end + 1;
            }
            b'\r' => {
                out.push(if attr { ' ' } else { '\n' });
                if bytes.get(i + 1) == Some(&b'\n') {
                    i += 1;
                }
                i += 1;
            }
            b'\n' | b'\t' if attr => {
                out.push(' ');
                i += 1;
            }
            _ => {
                // copy a run of ordinary bytes (keeps UTF-8 sequences intact)
                let start = i;
                while i < bytes.len()
                    && !matches!(bytes[i], b'&' | b'\r')
                    && !(attr && matches!(bytes[i], b'\n' | b'\t'))
                {
                    i += 1;
                }
                out.push_str(&raw[start..i]);
            }
        }
    }
    Ok(())
}

impl<'a> Parser<'a> {
    fn err<T>(&self, msg: &str) -> Result<T, String> {
        Err(format!("XML error at byte {}: {}", self.pos, msg))
    }

    fn starts_with(&self, s: &str) -> bool {
        self.b[self.pos..].starts_with(s.as_bytes())
    }

    fn skip_until(&mut self, pat: &str) -> Result<usize, String> {
        match memmem::find(&self.b[self.pos..], pat.as_bytes()) {
            Some(i) => {
                let start = self.pos;
                self.pos += i + pat.len();
                Ok(start + i)
            }
            None => self.err(&format!("expected '{}'", pat)),
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.b.len() && is_ws(self.b[self.pos]) {
            self.pos += 1;
        }
    }

    fn name(&mut self) -> &'a str {
        let start = self.pos;
        while self.pos < self.b.len() && !is_name_end(self.b[self.pos]) {
            self.pos += 1;
        }
        &self.src[start..self.pos]
    }

    /// Skip a DOCTYPE declaration (with an optional internal subset).
    fn skip_doctype(&mut self) -> Result<(), String> {
        let mut depth = 0;
        while self.pos < self.b.len() {
            match self.b[self.pos] {
                b'[' => depth += 1,
                b']' => depth -= 1,
                b'>' if depth == 0 => {
                    self.pos += 1;
                    return Ok(());
                }
                _ => {}
            }
            self.pos += 1;
        }
        self.err("unterminated DOCTYPE")
    }

    /// Parse attributes after the element name; returns true for `/>`.
    fn attributes(
        &mut self,
        attrs: Option<&mut Vec<(&'a str, Cow<'a, str>)>>,
    ) -> Result<bool, String> {
        let mut attrs = attrs;
        loop {
            self.skip_ws();
            if self.pos >= self.b.len() {
                return self.err("unterminated start tag");
            }
            match self.b[self.pos] {
                b'>' => {
                    self.pos += 1;
                    return Ok(false);
                }
                b'/' => {
                    if self.b.get(self.pos + 1) != Some(&b'>') {
                        return self.err("expected '/>'");
                    }
                    self.pos += 2;
                    return Ok(true);
                }
                _ => {}
            }
            let name = self.name();
            if name.is_empty() {
                return self.err("expected attribute name");
            }
            self.skip_ws();
            if self.b.get(self.pos) != Some(&b'=') {
                return self.err("expected '='");
            }
            self.pos += 1;
            self.skip_ws();
            let quote = match self.b.get(self.pos) {
                Some(&q @ (b'"' | b'\'')) => q,
                _ => return self.err("expected quoted attribute value"),
            };
            self.pos += 1;
            let end = match memchr(quote, &self.b[self.pos..]) {
                Some(i) => self.pos + i,
                None => return self.err("unterminated attribute value"),
            };
            if let Some(a) = attrs.as_deref_mut() {
                let src: &'a str = self.src;
                a.push((local(name), decode_cow(&src[self.pos..end], true)?));
            }
            self.pos = end + 1;
        }
    }

    /// Skip to after the end tag of the element `qname` whose start tag was
    /// just read, by searching for its (nested) start and end tags.
    fn skip_to_end_tag(&mut self, qname: &str) -> Result<(), String> {
        let finder = memmem::Finder::new(qname.as_bytes());
        let mut depth = 1;
        let mut from = self.pos;
        while depth > 0 {
            let i = match finder.find(&self.b[from..]) {
                Some(i) => from + i,
                None => return self.err("unterminated element"),
            };
            let after = self.b.get(i + qname.len()).copied().unwrap_or(b'>');
            let delimited = is_ws(after) || after == b'>' || after == b'/';
            from = i + qname.len();
            if !delimited {
                continue;
            }
            if i >= 2 && &self.b[i - 2..i] == b"</" {
                depth -= 1;
                if depth == 0 {
                    self.pos = from;
                    self.skip_ws();
                    if self.b.get(self.pos) != Some(&b'>') {
                        return self.err("expected '>'");
                    }
                    self.pos += 1;
                    return Ok(());
                }
            } else if i >= 1 && self.b[i - 1] == b'<' {
                // a nested element with the same name (unless self-closing)
                self.pos = from;
                if !self.attributes(None)? {
                    depth += 1;
                }
                from = self.pos;
            }
        }
        Ok(())
    }

    /// Skip the content of an element whose start tag was just read.
    #[allow(dead_code)]
    fn skip_element(&mut self) -> Result<(), String> {
        let mut depth = 1;
        while depth > 0 {
            let i = match memchr(b'<', &self.b[self.pos..]) {
                Some(i) => i,
                None => return self.err("unterminated element"),
            };
            self.pos += i;
            if self.starts_with("<!--") {
                self.skip_until("-->")?;
            } else if self.starts_with("<![CDATA[") {
                self.skip_until("]]>")?;
            } else if self.starts_with("<?") {
                self.skip_until("?>")?;
            } else if self.starts_with("</") {
                self.skip_until(">")?;
                depth -= 1;
            } else {
                self.pos += 1;
                self.name();
                if !self.attributes(None)? {
                    depth += 1;
                }
            }
        }
        Ok(())
    }

    fn parse(&mut self, skip: &[&str]) -> Result<Element<'a>, String> {
        let src: &'a str = self.src;
        let mut stack: Vec<Element<'a>> = Vec::new();
        let mut root: Option<Element<'a>> = None;
        while self.pos < self.b.len() {
            let next = memchr(b'<', &self.b[self.pos..])
                .map(|i| self.pos + i)
                .unwrap_or(self.b.len());
            if next > self.pos {
                let raw = &src[self.pos..next];
                if let Some(top) = stack.last_mut()
                    && !raw.bytes().all(is_ws)
                {
                    append_text(&mut top.text, raw, false)?;
                }
                self.pos = next;
                if self.pos >= self.b.len() {
                    break;
                }
            }
            if self.starts_with("<!--") {
                self.skip_until("-->")?;
            } else if self.starts_with("<![CDATA[") {
                self.pos += 9;
                let end = self.skip_until("]]>")?;
                if let Some(top) = stack.last_mut() {
                    append_text(&mut top.text, &src[next + 9..end], true)?;
                }
            } else if self.starts_with("<!") {
                self.skip_doctype()?;
            } else if self.starts_with("<?") {
                self.skip_until("?>")?;
            } else if self.starts_with("</") {
                self.pos += 2;
                let name = local(self.name());
                self.skip_ws();
                if self.b.get(self.pos) != Some(&b'>') {
                    return self.err("expected '>'");
                }
                self.pos += 1;
                let el = match stack.pop() {
                    Some(e) => e,
                    None => return self.err("unexpected end tag"),
                };
                if el.name != name {
                    return self.err(&format!("mismatched end tag </{}> for <{}>", name, el.name));
                }
                match stack.last_mut() {
                    Some(parent) => parent.children.push(el),
                    None => root = Some(el),
                }
            } else {
                self.pos += 1;
                let qname = self.name();
                let name = local(qname);
                if name.is_empty() {
                    return self.err("expected element name");
                }
                if root.is_some() && stack.is_empty() {
                    return self.err("content after the root element");
                }
                if !stack.is_empty() && skip.contains(&name) {
                    if !self.attributes(None)? {
                        self.skip_to_end_tag(qname)?;
                    }
                    continue;
                }
                let mut el = Element {
                    name,
                    ..Default::default()
                };
                let empty = self.attributes(Some(&mut el.attrs))?;
                if empty {
                    match stack.last_mut() {
                        Some(parent) => parent.children.push(el),
                        None => root = Some(el),
                    }
                } else {
                    stack.push(el);
                }
            }
        }
        if !stack.is_empty() {
            return self.err("unexpected end of document");
        }
        root.ok_or_else(|| "no root element".to_string())
    }
}

/// Parse an XML document, skipping the subtrees of the elements named in `skip`.
pub fn parse<'a>(xml: &'a str, skip: &[&str]) -> Result<Element<'a>, String> {
    let xml = xml.strip_prefix('\u{feff}').unwrap_or(xml);
    Parser {
        src: xml,
        b: xml.as_bytes(),
        pos: 0,
    }
    .parse(skip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_elements_attributes_and_text() {
        let doc = parse(
            "<?xml version=\"1.0\"?>\n<!-- c --><dmn:a x='1 &amp; 2' y=\"a\nb\"><b/><t>&lt;3 &#x41;<![CDATA[<raw>]]></t><skip><deep a='>'/></skip></dmn:a>",
            &["skip"],
        )
        .unwrap();
        assert_eq!(doc.name, "a");
        assert_eq!(doc.attribute("x"), Some("1 & 2"));
        assert_eq!(doc.attribute("y"), Some("a b"));
        assert_eq!(doc.children.len(), 2);
        assert_eq!(doc.children[1].text, "<3 A<raw>");
    }

    #[test]
    fn rejects_malformed() {
        assert!(parse("<a><b></a>", &[]).is_err());
        assert!(parse("<a>", &[]).is_err());
        assert!(parse("<a x=1/>", &[]).is_err());
    }
}
