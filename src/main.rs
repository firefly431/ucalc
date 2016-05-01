#![feature(box_patterns)]
#[macro_use]
extern crate nom;

use nom::{digit, IResult};

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
}

named!(parens<Expression>, dbg_dmp!(delimited!(char!('('), expr, char!(')'))));

named!(number<f64>, dbg_dmp!(map_res!(map_res!(digit, str::from_utf8), FromStr::from_str)));

named!(atom<Expression>, dbg_dmp!(alt!(number => {Expression::Value} | parens)));

named!(exp<Expression>, dbg_dmp!(chain!(lhs: atom ~ rhs: preceded!(char!('^'), exp)?, ||{
    match (lhs, rhs) {
        (lhs, None) => lhs,
        (Expression::Value(a), Some(Expression::Value(b))) => Expression::Value(a.powf(b)),
        (lhs, Some(b)) => Expression::Exp(Box::new(lhs), Box::new(b)),
    }
})));

named!(fac<Expression>, dbg_dmp!(
        chain!(first: exp
             ~ others: many0!(tuple!(
                       alt!(char!('*') | char!('/')), exp)), ||
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
                       alt!(char!('+') | char!('-')), fac)), ||
    others.into_iter().fold(first, |lhs, (op, rhs)| simplify1(
            match op {
                '+' => Expression::Add(Box::new(lhs), Box::new(rhs)),
                '-' => Expression::Sub(Box::new(lhs), Box::new(rhs)),
                _   => Expression::Add(Box::new(lhs), Box::new(rhs))
            }))
)));

named!(input<Expression>, dbg_dmp!(chain!(res: expr ~ char!('?'), ||{res})));

fn simplify1(expr: Expression) -> Expression {
    match expr {
        Expression::Exp(box Expression::Value(a), box Expression::Value(b)) => Expression::Value(a.powf(b)),
        Expression::Mul(box Expression::Value(a), box Expression::Value(b)) => Expression::Value(a * b),
        Expression::Div(box Expression::Value(a), box Expression::Value(b)) => Expression::Value(a / b),
        Expression::Add(box Expression::Value(a), box Expression::Value(b)) => Expression::Value(a + b),
        Expression::Sub(box Expression::Value(a), box Expression::Value(b)) => Expression::Value(a - b),
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
}

#[test]
fn test_addsub() {
    test_expr!("1+1", 2.0);
    test_expr!("3-2", 1.0);
    test_expr!("3-2+3", 4.0);
    test_expr!("2^3*4-5", 27.0);
}

fn main() {
    println!("A: {:?}", input(b"1?"));
    println!("A: {:?}", input(b"1*1?"));
    println!("A: {:?}", input(b"1/2*3?"));
    println!("A: {:?}", input(b"1^1?"));
    println!("A: {:?}", input(b"1^1^1?"));
    println!("Result: {:?}", input(b"2/3^5*2^1^2?"));
}
