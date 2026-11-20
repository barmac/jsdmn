//! FEEL (Friendly Enough Expression Language).

pub mod ast;
pub mod builtins;
pub mod eval;
pub mod lexer;
pub mod optimize;
pub mod parser;
pub mod regex;

use crate::value::Value;
use eval::Scope;
use parser::EmptyEnv;

/// Evaluate a standalone FEEL expression with the given variables.
pub fn evaluate(expr: &str, vars: &[(&str, Value)]) -> Result<Value, String> {
    let names: Vec<String> = vars.iter().map(|(n, _)| lexer::normalize_name(n)).collect();
    let e = parser::parse_expression_with(expr, &EmptyEnv, &names).map_err(|e| e.0)?;
    let e = optimize::fold_expr(e, &|n| names.iter().any(|x| x == n));
    let scope = Scope::root();
    for (n, v) in vars {
        scope.set(&lexer::normalize_name(n), v.clone());
    }
    Ok(eval::eval(&e, &scope))
}
