#![feature(box_patterns)]
#[macro_use]
extern crate nom;

use nom::{digit, multispace, IResult};

use std::str;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
enum Expression {
    Value(f64),
    Exp(Box<Expression>, Box<Expression>),
    Mul(Box<Expression>, Box<Expression>),
    Div(Box<Expression>, Box<Expression>),
    Add(Box<Expression>, Box<Expression>),
    Sub(Box<Expression>, Box<Expression>),
    Neg(Box<Expression>),
}

named!(parens<Expression>, dbg_dmp!(delimited!(char!('('), preceded!(opt!(multispace), expr), preceded!(opt!(multispace), char!(')')))));

named!(number<f64>, dbg_dmp!(map_res!(map_res!(digit, str::from_utf8), FromStr::from_str)));

named!(atom<Expression>, dbg_dmp!(alt!(number => {Expression::Value} | parens)));

named!(imul<Expression>, dbg_dmp!(chain!(first: atom ~ others: many0!(atom), ||
    others.into_iter().fold(first, |lhs, rhs| simplify1(Expression::Mul(Box::new(lhs), Box::new(rhs))))
)));

named!(unary<Expression>, dbg_dmp!(alt!(exp | chain!(op: chain!(o: alt!(char!('+') | char!('-')) ~ multispace?, || o) ~ val: unary, ||{
    match op {
        '+' => val,
        '-' => simplify1(Expression::Neg(Box::new(val))),
        _ => val,
    }
}))));

named!(exp<Expression>, dbg_dmp!(chain!(lhs: imul ~ rhs: preceded!(preceded!(opt!(multispace), char!('^')), preceded!(opt!(multispace), unary))?, ||{
    match (lhs, rhs) {
        (lhs, None) => lhs,
        (Expression::Value(a), Some(Expression::Value(b))) => Expression::Value(a.powf(b)),
        (lhs, Some(b)) => Expression::Exp(Box::new(lhs), Box::new(b)),
    }
})));

named!(facterm<(char, Expression)>,
        tuple!(alt!(
                preceded!(opt!(multispace), char!('*'))
              | preceded!(opt!(multispace), char!('/'))
              | value!('*', preceded!(multispace, error!(nom::ErrorKind::NoneOf, peek!(none_of!("+-")))))), preceded!(opt!(multispace), unary)));

named!(fac<Expression>, dbg_dmp!(
        chain!(first: unary
             ~ others: many0!(facterm), ||
    others.into_iter().fold(first, |lhs, (op, rhs)| simplify1(
            match op {
                '*' => Expression::Mul(Box::new(lhs), Box::new(rhs)),
                '/' => Expression::Div(Box::new(lhs), Box::new(rhs)),
                _   => Expression::Mul(Box::new(lhs), Box::new(rhs))
            }))
)));

named!(expr<Expression>, dbg_dmp!(
        chain!(first: fac
             ~ others: many0!(tuple!(
                       preceded!(opt!(multispace), alt!(char!('+') | char!('-'))), preceded!(opt!(multispace), fac))), ||
    others.into_iter().fold(first, |lhs, (op, rhs)| simplify1(
            match op {
                '+' => Expression::Add(Box::new(lhs), Box::new(rhs)),
                '-' => Expression::Sub(Box::new(lhs), Box::new(rhs)),
                _   => Expression::Add(Box::new(lhs), Box::new(rhs))
            }))
)));

named!(input<Expression>, dbg_dmp!(chain!(opt!(multispace) ~ res: expr ~ opt!(multispace) ~ char!('?'), ||{res})));

fn simplify1(expr: Expression) -> Expression {
    match expr {
        Expression::Exp(box Expression::Value(a), box Expression::Value(b)) => Expression::Value(a.powf(b)),
        Expression::Mul(box Expression::Value(a), box Expression::Value(b)) => Expression::Value(a * b),
        Expression::Div(box Expression::Value(a), box Expression::Value(b)) => Expression::Value(a / b),
        Expression::Add(box Expression::Value(a), box Expression::Value(b)) => Expression::Value(a + b),
        Expression::Sub(box Expression::Value(a), box Expression::Value(b)) => Expression::Value(a - b),
        Expression::Neg(box Expression::Value(a)) => Expression::Value(-a),
        expr => expr
    }
}

macro_rules! test_expr {
    ( $x:expr, $v: expr) => (assert_eq!(input(concat!($x, "?").as_bytes()), IResult::Done(&b""[..], Expression::Value($v))));
}

#[test]
fn test_exponents() {
    test_expr!("2^1^5", 2.0);
}

#[test]
fn test_muldiv() {
    test_expr!("2*3", 6.0);
    test_expr!("3/2", 1.5);
    test_expr!("3/2*4", 6.0);
    test_expr!("2^2*3", 12.0);
    test_expr!("2 2 2 ", 8.0);
}

#[test]
fn test_implied_mul() {
    test_expr!("1/2(4)", 0.125);
    test_expr!("1/2 (4)", 2.0);
    test_expr!("1(2)3(4)5(6)7(8)9(10)", 3628800.0)
}

#[test]
fn test_addsub() {
    test_expr!("1+1", 2.0);
    test_expr!("3-2", 1.0);
    test_expr!("3-2+3", 4.0);
    test_expr!("2^3*4-5", 27.0);
}

#[test]
fn test_whitespace() {
    test_expr!(" (2^39)* 122/2 + 80 -1023 ", 33535104646225.0);
    test_expr!("(    2     ^   1   )   * 5    / 2 +   3    - 5", 3.0);
}

#[test]
fn test_huge() {
    test_expr!("(((17 - 9 - 14) / 1 + 13 * 15) / 5 / 8 - 18) / 11 * 15 * 17 / (16 / 5 + 10 * 16 / ((5 / 14 - 3 - 4 - 6) * (9 * 7 / 2 - 7 - 16)))", -179.844926355302559466636533137465393525057912876433696);
}

#[test]
fn test_unary() {
    test_expr!("1+-1(2)", -1.0);
    test_expr!("1/2-2", -1.5);
    test_expr!("1+1", 2.0);
    test_expr!("1 + 1", 2.0);
    test_expr!("1+1/-(3-2)", 0.0);
    test_expr!("-2^2", -4.0);
    test_expr!("2^-2", 0.25);
    test_expr!("-2(5)", -10.0);
}

fn main() {
    println!("A: {:?}", input(b"1?"));
    println!("A: {:?}", input(b"1*1?"));
    println!("A: {:?}", input(b"1/2*3?"));
    println!("A: {:?}", input(b"1^1?"));
    println!("A: {:?}", input(b"1^1^1?"));
    println!("Result: {:?}", input(b"2/3^5*2^1^2?"));
}
