//! FEEL expression smoke tests.

use rsdmn::feel::evaluate;
use rsdmn::value::Value;

fn eval(expr: &str) -> String {
    format!("{:?}", evaluate(expr, &[]).unwrap())
}

#[test]
fn arithmetic() {
    assert_eq!(eval("1 + 2 * 3"), "7");
    assert_eq!(eval("1 / 3"), "0.3333333333333333333333333333333333");
    assert_eq!(eval("-3 ** 2"), "9");
    assert_eq!(eval("3 ** 4 ** 5"), "3486784401");
    assert_eq!(eval("10 / 0"), "null");
}

#[test]
fn names_with_spaces() {
    let v = evaluate(
        "Monthly Salary * 12",
        &[("Monthly Salary", Value::int(1000))],
    )
    .unwrap();
    assert_eq!(format!("{:?}", v), "12000");
    assert_eq!(
        eval("{first name: \"a\", greeting: \"hi \" + first name}.greeting"),
        "\"hi a\""
    );
}

#[test]
fn lists_and_iteration() {
    assert_eq!(eval("for x in 1..3 return x * 2"), "[2, 4, 6]");
    assert_eq!(eval("[1, 2, 3, 4][item > 2]"), "[3, 4]");
    assert_eq!(eval("some x in [1, 2, 3] satisfies x > 2"), "true");
    assert_eq!(eval("sum([1, 2, 3])"), "6");
    assert_eq!(eval("sort([3, 1, 2], function(a, b) a < b)"), "[1, 2, 3]");
}

#[test]
fn temporal() {
    assert_eq!(
        eval("date(\"2021-01-31\") + duration(\"P1M\")"),
        "date(\"2021-02-28\")"
    );
    assert_eq!(
        eval("@\"2021-01-02\" - @\"2021-01-01T10:10:10+11:00\""),
        "duration(\"P1DT49M50S\")"
    );
    assert_eq!(eval("day of week(date(\"2018-12-08\"))"), "\"Saturday\"");
}

#[test]
fn ranges_and_unary_tests() {
    assert_eq!(eval("5 in [1..10]"), "true");
    assert_eq!(eval("5 in (< 3, > 4)"), "true");
    assert_eq!(eval("before(1, [2..5])"), "true");
}

#[test]
fn strings() {
    assert_eq!(
        eval("replace(\"abcd\", \"(ab)|(a)\", \"[1=$1][2=$2]\")"),
        "\"[1=ab][2=]cd\""
    );
    assert_eq!(
        eval("split(\"a;b;c;;\", \";\")"),
        "[\"a\", \"b\", \"c\", \"\", \"\"]"
    );
    assert_eq!(eval("substring(\"foobar\", -2, 1)"), "\"a\"");
}
