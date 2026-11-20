//! FEEL parser (recursive descent with scope aware name recognition).

use super::ast::*;
use super::lexer::{Tok, Token, normalize_name, tokenize};
use crate::number::Number;
use crate::temporal::{self, Date, DateTime, Duration, Time};
use crate::types::{self, DmnType};
use crate::value::{CmpOp, Value};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct ParseError(pub String);

type PResult<T> = Result<T, ParseError>;

/// Information about the surrounding model used during parsing.
pub trait ParseEnv {
    /// Is `n` a (normalized) name known in the model?
    fn is_known_name(&self, n: &str) -> bool;
    /// Resolve a named type (item definition or builtin).
    fn resolve_type(&self, n: &str) -> Option<Rc<DmnType>>;
}

/// Environment without model information.
pub struct EmptyEnv;

impl ParseEnv for EmptyEnv {
    fn is_known_name(&self, _n: &str) -> bool {
        false
    }
    fn resolve_type(&self, n: &str) -> Option<Rc<DmnType>> {
        types::builtin_type(n).map(Rc::new)
    }
}

const STOP_WORDS: &[&str] = &[
    "and",
    "or",
    "in",
    "between",
    "instance",
    "then",
    "else",
    "return",
    "satisfies",
];

struct Parser<'a> {
    src: &'a str,
    toks: Vec<Token>,
    pos: usize,
    env: &'a dyn ParseEnv,
    locals: Vec<String>,
}

fn cmp_op(t: &Token) -> Option<CmpOp> {
    match &t.tok {
        Tok::Punct("=") => Some(CmpOp::Eq),
        Tok::Punct("!=") => Some(CmpOp::Ne),
        Tok::Punct("<") => Some(CmpOp::Lt),
        Tok::Punct("<=") => Some(CmpOp::Le),
        Tok::Punct(">") => Some(CmpOp::Gt),
        Tok::Punct(">=") => Some(CmpOp::Ge),
        _ => None,
    }
}

pub fn parse_at_literal(s: &str) -> Option<Value> {
    if s.starts_with('P') || s.starts_with("-P") {
        return match temporal::parse_duration(s)? {
            Duration::Dt(d) => Some(Value::DtDuration(d)),
            Duration::Ym(d) => Some(Value::YmDuration(d)),
        };
    }
    if s.contains('T') {
        return DateTime::parse(s).map(Value::DateTime);
    }
    if s.len() >= 3 && s.as_bytes()[2] == b':' {
        return Time::parse(s).map(Value::Time);
    }
    Date::parse(s).map(Value::Date)
}

impl<'a> Parser<'a> {
    fn new(src: &'a str, env: &'a dyn ParseEnv) -> PResult<Parser<'a>> {
        let toks = tokenize(src).map_err(|e| ParseError(e.0))?;
        Ok(Parser {
            src,
            toks,
            pos: 0,
            env,
            locals: Vec::new(),
        })
    }

    fn peek(&self) -> &Token {
        &self.toks[self.pos]
    }

    fn peek_at(&self, i: usize) -> &Token {
        let i = (self.pos + i).min(self.toks.len() - 1);
        &self.toks[i]
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek().tok, Tok::Eof)
    }

    fn err<T>(&self, msg: &str) -> PResult<T> {
        let t = self.peek();
        Err(ParseError(format!(
            "{} at {} ('{}') in `{}`",
            msg,
            t.start,
            &self.src[t.start..t.end.max(t.start)],
            self.src
        )))
    }

    fn eat(&mut self, p: &str) -> bool {
        if self.peek().is_punct(p) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn eat_name(&mut self, n: &str) -> bool {
        if self.peek().is_name(n) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, p: &str) -> PResult<()> {
        if self.eat(p) {
            Ok(())
        } else {
            self.err(&format!("expected '{}'", p))
        }
    }

    fn expect_name(&mut self, n: &str) -> PResult<()> {
        if self.eat_name(n) {
            Ok(())
        } else {
            self.err(&format!("expected '{}'", n))
        }
    }

    fn is_known(&self, n: &str) -> bool {
        self.locals.iter().any(|l| l == n)
            || self.env.is_known_name(n)
            || super::builtins::is_builtin_name(n)
            || types::builtin_type(n).is_some()
    }

    fn slice(&self, from: usize, to: usize) -> String {
        normalize_name(&self.src[self.toks[from].start..self.toks[to].end])
    }

    /// Length (in tokens) of the longest known name starting at the current position.
    fn known_match_len(&self) -> usize {
        let start = self.pos;
        let mut best = 0;
        let max = (start + 24).min(self.toks.len() - 1);
        for j in start..max {
            match &self.toks[j].tok {
                Tok::Eof | Tok::Str(_) | Tok::At(_) => break,
                Tok::Punct(p) if !matches!(*p, "." | "-" | "+" | "*" | "/") => break,
                _ => {}
            }
            if matches!(self.toks[start].tok, Tok::Name(_)) && self.is_known(&self.slice(start, j))
            {
                best = j - start + 1;
            }
        }
        best
    }

    fn parse_name(&mut self) -> PResult<String> {
        let start = self.pos;
        if self.peek().name().is_none() {
            return self.err("expected name");
        }
        let known = self.known_match_len();
        let mut end = if known > 0 { start + known - 1 } else { start };
        while let Some(n) = self.toks[end + 1].name() {
            if STOP_WORDS.contains(&n) {
                break;
            }
            end += 1;
        }
        self.pos = end + 1;
        Ok(self.slice(start, end))
    }

    /// Raw name made of all tokens up to one of the stop punctuations.
    fn parse_name_until(&mut self, stops: &[&str], stop_words: &[&str]) -> PResult<String> {
        let start = self.pos;
        loop {
            let t = self.peek();
            match &t.tok {
                Tok::Eof => break,
                Tok::Punct(p) if stops.contains(p) => break,
                Tok::Name(n) if stop_words.contains(&n.as_str()) && self.pos > start => break,
                _ => self.pos += 1,
            }
        }
        if self.pos == start {
            return self.err("expected name");
        }
        Ok(self.slice(start, self.pos - 1))
    }

    fn parse_expr(&mut self) -> PResult<Expr> {
        let t = self.peek().clone();
        if let Tok::Name(n) = &t.tok
            && self.known_match_len() <= 1
        {
            match n.as_str() {
                "if" => return self.parse_if(),
                "for" => return self.parse_for(),
                "some" => return self.parse_quantified(false),
                "every" => return self.parse_quantified(true),
                _ => {}
            }
        }
        self.parse_or()
    }

    fn parse_if(&mut self) -> PResult<Expr> {
        self.expect_name("if")?;
        let c = self.parse_expr()?;
        self.expect_name("then")?;
        let a = self.parse_expr()?;
        self.expect_name("else")?;
        let b = self.parse_expr()?;
        Ok(Expr::If(Box::new(c), Box::new(a), Box::new(b)))
    }

    fn parse_for(&mut self) -> PResult<Expr> {
        self.expect_name("for")?;
        let saved = self.locals.len();
        let mut iters = Vec::new();
        loop {
            let name = self.parse_name_until(&[], &["in"])?;
            self.expect_name("in")?;
            let e = self.parse_expr()?;
            let it = if self.eat("..") {
                let end = self.parse_expr()?;
                Iteration::Range(e, end)
            } else {
                Iteration::Expr(e)
            };
            self.locals.push(name.clone());
            iters.push((Rc::from(name.as_str()), it));
            if self.eat(",") {
                continue;
            }
            break;
        }
        self.expect_name("return")?;
        self.locals.push("partial".into());
        let body = self.parse_expr()?;
        self.locals.truncate(saved);
        Ok(Expr::For(iters, Box::new(body)))
    }

    fn parse_quantified(&mut self, every: bool) -> PResult<Expr> {
        self.pos += 1;
        let saved = self.locals.len();
        let mut iters = Vec::new();
        loop {
            let name = self.parse_name_until(&[], &["in"])?;
            self.expect_name("in")?;
            let e = self.parse_or()?;
            self.locals.push(name.clone());
            iters.push((Rc::from(name.as_str()), e));
            if self.eat(",") {
                continue;
            }
            break;
        }
        self.expect_name("satisfies")?;
        let cond = self.parse_expr()?;
        self.locals.truncate(saved);
        Ok(Expr::Quantified {
            every,
            iters,
            cond: Box::new(cond),
        })
    }

    fn parse_function_def(&mut self) -> PResult<Expr> {
        self.expect_name("function")?;
        self.expect("(")?;
        let mut params: Vec<(Rc<str>, Option<Rc<DmnType>>)> = Vec::new();
        if !self.eat(")") {
            loop {
                let name = self.parse_name_until(&[":", ",", ")"], &[])?;
                let ty = if self.eat(":") {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                params.push((Rc::from(name.as_str()), ty));
                if self.eat(",") {
                    continue;
                }
                self.expect(")")?;
                break;
            }
        }
        let kind = if self.eat_name("external") {
            FnKind::External
        } else {
            FnKind::Feel
        };
        let saved = self.locals.len();
        for (p, _) in &params {
            self.locals.push(p.to_string());
        }
        let body = self.parse_expr()?;
        self.locals.truncate(saved);
        Ok(Expr::FunctionDef(Rc::new(FunctionDef {
            params,
            body,
            kind,
            result_type: None,
        })))
    }

    fn parse_or(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_and()?;
        while self.eat_name("or") {
            let rhs = self.parse_and()?;
            lhs = Expr::Or(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_comparison()?;
        while self.eat_name("and") {
            let rhs = self.parse_comparison()?;
            lhs = Expr::And(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_comparison(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_additive()?;
        loop {
            if let Some(op) = cmp_op(self.peek()) {
                self.pos += 1;
                let rhs = self.parse_additive()?;
                lhs = Expr::Cmp(op, Box::new(lhs), Box::new(rhs));
                continue;
            }
            if self.eat_name("between") {
                let lo = self.parse_additive()?;
                self.expect_name("and")?;
                let hi = self.parse_additive()?;
                lhs = Expr::Between(Box::new(lhs), Box::new(lo), Box::new(hi));
                continue;
            }
            if self.peek().is_name("in") {
                self.pos += 1;
                let tests = self.parse_in_rhs()?;
                lhs = Expr::In(Box::new(lhs), Rc::new(tests));
                continue;
            }
            if self.peek().is_name("instance") && self.peek_at(1).is_name("of") {
                self.pos += 2;
                let ty = self.parse_type()?;
                lhs = Expr::InstanceOf(Box::new(lhs), ty);
                continue;
            }
            break;
        }
        Ok(lhs)
    }

    /// Does the parenthesized group starting at the current `(` contain a top level comma?
    fn paren_group_has_comma(&self) -> bool {
        let mut depth = 0;
        let mut i = self.pos;
        while i < self.toks.len() {
            match &self.toks[i].tok {
                Tok::Punct("(") | Tok::Punct("[") | Tok::Punct("{") => depth += 1,
                Tok::Punct(")") | Tok::Punct("]") | Tok::Punct("}") => {
                    depth -= 1;
                    if depth == 0 {
                        return false;
                    }
                }
                Tok::Punct(",") if depth == 1 => return true,
                Tok::Eof => return false,
                _ => {}
            }
            i += 1;
        }
        false
    }

    fn parse_in_rhs(&mut self) -> PResult<UnaryTests> {
        if self.peek().is_punct("(") && self.paren_group_has_comma() {
            self.pos += 1;
            let mut tests = Vec::new();
            loop {
                tests.push(self.parse_positive_unary_test(false)?);
                if self.eat(",") {
                    continue;
                }
                self.expect(")")?;
                break;
            }
            return Ok(UnaryTests {
                tests,
                negated: false,
            });
        }
        let t = self.parse_positive_unary_test(false)?;
        Ok(UnaryTests {
            tests: vec![t],
            negated: false,
        })
    }

    fn parse_positive_unary_test(&mut self, full: bool) -> PResult<UnaryTest> {
        if let Some(op) = cmp_op(self.peek()) {
            self.pos += 1;
            let e = self.parse_additive()?;
            return Ok(UnaryTest::Cmp(op, e));
        }
        let expr = if full {
            self.parse_expr()?
        } else {
            self.parse_additive()?
        };
        let uses_qmark = expr.uses_name("?");
        Ok(UnaryTest::Expr { expr, uses_qmark })
    }

    fn parse_additive(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = if self.eat("+") {
                ArithOp::Add
            } else if self.eat("-") {
                ArithOp::Sub
            } else {
                break;
            };
            let rhs = self.parse_multiplicative()?;
            lhs = Expr::Arith(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_power()?;
        loop {
            let op = if self.eat("*") {
                ArithOp::Mul
            } else if self.eat("/") {
                ArithOp::Div
            } else {
                break;
            };
            let rhs = self.parse_power()?;
            lhs = Expr::Arith(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_power(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_unary()?;
        while self.eat("**") {
            let rhs = self.parse_unary()?;
            lhs = Expr::Arith(ArithOp::Pow, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        if self.eat("-") {
            let e = self.parse_unary()?;
            if let Expr::Const(Value::Number(n)) = &e {
                return Ok(Expr::Const(Value::Number(n.neg())));
            }
            return Ok(Expr::Neg(Box::new(e)));
        }
        if self.eat("+") {
            return self.parse_unary();
        }
        self.parse_postfix()
    }

    fn can_start_expr(&self, t: &Token) -> bool {
        match &t.tok {
            Tok::Number(_) | Tok::Str(_) | Tok::At(_) => true,
            Tok::Name(n) => !STOP_WORDS.contains(&n.as_str()),
            Tok::Punct(p) => matches!(
                *p,
                "(" | "[" | "{" | "-" | "<" | "<=" | ">" | ">=" | "=" | "!=" | "]"
            ),
            Tok::Eof => false,
        }
    }

    fn parse_postfix(&mut self) -> PResult<Expr> {
        let mut e = self.parse_primary()?;
        loop {
            if self.peek().is_punct(".") && self.peek_at(1).name().is_some() {
                self.pos += 1;
                let name = self.parse_name()?;
                e = Expr::Path(Box::new(e), Rc::from(name.as_str()));
            } else if self.peek().is_punct("[")
                && self.can_start_expr(self.peek_at(1))
                && !self.peek_at(1).is_punct("]")
            {
                self.pos += 1;
                let saved = self.locals.len();
                self.locals.push("item".into());
                let f = self.parse_expr()?;
                self.locals.truncate(saved);
                self.expect("]")?;
                e = Expr::Filter(Box::new(e), Box::new(f));
            } else if self.peek().is_punct("(") {
                self.pos += 1;
                let args = self.parse_args()?;
                e = Expr::Call(Box::new(e), args);
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn is_named_arg_start(&self) -> bool {
        let mut j = self.pos;
        while self.toks[j].name().is_some() {
            j += 1;
        }
        j > self.pos && self.toks[j].is_punct(":")
    }

    fn parse_args(&mut self) -> PResult<Args> {
        if self.eat(")") {
            return Ok(Args::Positional(Vec::new()));
        }
        if self.is_named_arg_start() {
            let mut args = Vec::new();
            loop {
                let name = self.parse_name_until(&[":"], &[])?;
                self.expect(":")?;
                let e = self.parse_expr()?;
                args.push((Rc::from(name.as_str()), e));
                if self.eat(",") {
                    continue;
                }
                self.expect(")")?;
                break;
            }
            return Ok(Args::Named(args));
        }
        let mut args = Vec::new();
        loop {
            args.push(self.parse_expr()?);
            if self.eat(",") {
                continue;
            }
            self.expect(")")?;
            break;
        }
        Ok(Args::Positional(args))
    }

    fn range_close(&mut self) -> PResult<bool> {
        if self.eat("]") {
            Ok(true)
        } else if self.eat(")") || self.eat("[") {
            Ok(false)
        } else {
            self.err("expected range end")
        }
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        let t = self.peek().clone();
        match &t.tok {
            Tok::Number(s) => {
                self.pos += 1;
                match Number::parse(s) {
                    Some(n) => Ok(Expr::Const(Value::Number(n))),
                    None => self.err("bad number"),
                }
            }
            Tok::Str(s) => {
                self.pos += 1;
                Ok(Expr::Const(Value::str(s)))
            }
            Tok::At(s) => {
                self.pos += 1;
                match parse_at_literal(s) {
                    Some(v) => Ok(Expr::Const(v)),
                    None => Ok(Expr::Error(format!("invalid @ literal \"{}\"", s))),
                }
            }
            Tok::Punct("(") => {
                self.pos += 1;
                let e = self.parse_expr()?;
                if self.eat("..") {
                    let end = self.parse_expr()?;
                    let end_incl = self.range_close()?;
                    return Ok(Expr::Range {
                        start: Box::new(e),
                        start_incl: false,
                        end: Box::new(end),
                        end_incl,
                    });
                }
                self.expect(")")?;
                Ok(e)
            }
            Tok::Punct("[") => {
                self.pos += 1;
                if self.eat("]") {
                    return Ok(Expr::List(Vec::new()));
                }
                let e = self.parse_expr()?;
                if self.eat("..") {
                    let end = self.parse_expr()?;
                    let end_incl = self.range_close()?;
                    return Ok(Expr::Range {
                        start: Box::new(e),
                        start_incl: true,
                        end: Box::new(end),
                        end_incl,
                    });
                }
                let mut items = vec![e];
                while self.eat(",") {
                    items.push(self.parse_expr()?);
                }
                self.expect("]")?;
                Ok(Expr::List(items))
            }
            Tok::Punct("]") => {
                self.pos += 1;
                let e = self.parse_expr()?;
                self.expect("..")?;
                let end = self.parse_expr()?;
                let end_incl = self.range_close()?;
                Ok(Expr::Range {
                    start: Box::new(e),
                    start_incl: false,
                    end: Box::new(end),
                    end_incl,
                })
            }
            Tok::Punct("{") => self.parse_context(),
            Tok::Punct(_) if cmp_op(&t).is_some() => {
                let op = cmp_op(&t).unwrap();
                self.pos += 1;
                let e = self.parse_additive()?;
                Ok(Expr::UnaryRange(op, Box::new(e)))
            }
            Tok::Name(n) => {
                if self.known_match_len() <= 1 {
                    match n.as_str() {
                        "if" => return self.parse_if(),
                        "for" => return self.parse_for(),
                        "some" => return self.parse_quantified(false),
                        "every" => return self.parse_quantified(true),
                        "function" if self.peek_at(1).is_punct("(") => {
                            return self.parse_function_def();
                        }
                        "true" => {
                            self.pos += 1;
                            return Ok(Expr::Const(Value::Bool(true)));
                        }
                        "false" => {
                            self.pos += 1;
                            return Ok(Expr::Const(Value::Bool(false)));
                        }
                        "null" => {
                            self.pos += 1;
                            return Ok(Expr::Const(Value::Null));
                        }
                        _ => {}
                    }
                }
                let name = self.parse_name()?;
                Ok(Expr::Name(Rc::from(name.as_str())))
            }
            _ => self.err("unexpected token"),
        }
    }

    fn parse_context(&mut self) -> PResult<Expr> {
        self.expect("{")?;
        let mut entries = Vec::new();
        if self.eat("}") {
            return Ok(Expr::Context(entries));
        }
        let saved = self.locals.len();
        loop {
            let key = if let Tok::Str(s) = &self.peek().tok {
                let s = s.clone();
                self.pos += 1;
                s
            } else {
                self.parse_name_until(&[":", ",", "}"], &[])?
            };
            self.expect(":")?;
            self.locals.push(key.clone());
            let e = self.parse_expr()?;
            entries.push(ContextEntry {
                name: Some(Rc::from(key.as_str())),
                expr: e,
            });
            if self.eat(",") {
                continue;
            }
            self.expect("}")?;
            break;
        }
        self.locals.truncate(saved);
        Ok(Expr::Context(entries))
    }

    fn parse_type(&mut self) -> PResult<Rc<DmnType>> {
        let t = self.peek().clone();
        if let Some(n) = t.name() {
            if self.peek_at(1).is_punct("<") {
                match n {
                    "list" => {
                        self.pos += 2;
                        let inner = self.parse_type()?;
                        self.expect(">")?;
                        return Ok(Rc::new(DmnType::List(inner)));
                    }
                    "range" => {
                        self.pos += 2;
                        let inner = self.parse_type()?;
                        self.expect(">")?;
                        return Ok(Rc::new(DmnType::Range(inner)));
                    }
                    "context" => {
                        self.pos += 2;
                        let mut entries = Vec::new();
                        if !self.eat(">") {
                            loop {
                                let name = self.parse_name_until(&[":"], &[])?;
                                self.expect(":")?;
                                let ty = self.parse_type()?;
                                entries.push((Rc::from(name.as_str()), ty));
                                if self.eat(",") {
                                    continue;
                                }
                                self.expect(">")?;
                                break;
                            }
                        }
                        return Ok(Rc::new(DmnType::Context(entries)));
                    }
                    "function" => {
                        self.pos += 2;
                        let mut params = Vec::new();
                        if !self.eat(">") {
                            loop {
                                params.push(self.parse_type()?);
                                if self.eat(",") {
                                    continue;
                                }
                                self.expect(">")?;
                                break;
                            }
                        }
                        self.expect("->")?;
                        let ret = self.parse_type()?;
                        return Ok(Rc::new(DmnType::Function(params, ret)));
                    }
                    _ => {}
                }
            }
            // longest known type name
            let start = self.pos;
            let mut best: Option<(usize, Rc<DmnType>)> = None;
            let max = (start + 12).min(self.toks.len() - 1);
            for j in start..max {
                match &self.toks[j].tok {
                    Tok::Name(_) => {}
                    Tok::Punct(".") | Tok::Punct("-") => {}
                    _ => break,
                }
                let s = self.slice(start, j);
                if let Some(ty) = self.env.resolve_type(&s) {
                    best = Some((j, ty));
                }
            }
            if let Some((j, ty)) = best {
                self.pos = j + 1;
                return Ok(ty);
            }
            self.pos += 1;
            return Ok(Rc::new(DmnType::Unknown(n.to_string())));
        }
        self.err("expected type")
    }
}

fn finish<T>(p: &Parser, v: T) -> PResult<T> {
    if !p.at_eof() {
        return p.err("unexpected trailing input");
    }
    Ok(v)
}

pub fn parse_expression(src: &str, env: &dyn ParseEnv) -> PResult<Expr> {
    let mut p = Parser::new(src, env)?;
    if p.at_eof() {
        return Ok(Expr::Const(Value::Null));
    }
    let e = p.parse_expr()?;
    finish(&p, e)
}

/// Parse an expression with extra names in scope.
pub fn parse_expression_with(src: &str, env: &dyn ParseEnv, locals: &[String]) -> PResult<Expr> {
    let mut p = Parser::new(src, env)?;
    p.locals.extend(locals.iter().cloned());
    if p.at_eof() {
        return Ok(Expr::Const(Value::Null));
    }
    let e = p.parse_expr()?;
    finish(&p, e)
}

pub fn parse_type_str(src: &str, env: &dyn ParseEnv) -> PResult<Rc<DmnType>> {
    let mut p = Parser::new(src, env)?;
    let t = p.parse_type()?;
    finish(&p, t)
}

pub fn parse_unary_tests(src: &str, env: &dyn ParseEnv) -> PResult<UnaryTests> {
    let mut p = Parser::new(src, env)?;
    if p.at_eof() {
        return Ok(UnaryTests {
            tests: vec![UnaryTest::Any],
            negated: false,
        });
    }
    if p.peek().is_punct("-") && matches!(p.peek_at(1).tok, Tok::Eof) {
        return Ok(UnaryTests {
            tests: vec![UnaryTest::Any],
            negated: false,
        });
    }
    let mut negated = false;
    if p.peek().is_name("not") && p.peek_at(1).is_punct("(") {
        // find matching paren
        let mut depth = 0;
        let mut i = p.pos + 1;
        let mut close = None;
        while i < p.toks.len() {
            match &p.toks[i].tok {
                Tok::Punct("(") | Tok::Punct("[") | Tok::Punct("{") => depth += 1,
                Tok::Punct(")") | Tok::Punct("]") | Tok::Punct("}") => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if let Some(c) = close
            && matches!(p.toks[c + 1].tok, Tok::Eof)
            && p.toks[c].is_punct(")")
        {
            negated = true;
            p.pos += 2;
            let mut tests = Vec::new();
            loop {
                tests.push(p.parse_positive_unary_test(true)?);
                if p.eat(",") {
                    continue;
                }
                p.expect(")")?;
                break;
            }
            return finish(&p, UnaryTests { tests, negated });
        }
    }
    let mut tests = Vec::new();
    loop {
        if p.peek().is_punct("-")
            && (p.peek_at(1).is_punct(",") || matches!(p.peek_at(1).tok, Tok::Eof))
        {
            p.pos += 1;
            tests.push(UnaryTest::Any);
        } else {
            tests.push(p.parse_positive_unary_test(true)?);
        }
        if p.eat(",") {
            continue;
        }
        break;
    }
    finish(&p, UnaryTests { tests, negated })
}
