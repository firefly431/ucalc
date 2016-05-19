//! The main program file. Contains the main function and ties together all the submodules.
//!
//! Note regarding documentation: Rust does not currently support documentation on items
//! generated by macros, so many functions in this module are undocumented. Read the source
//! code to see a few notes on these items.

#![feature(box_patterns)]
#![feature(plugin)]
#![plugin(phf_macros)]
#[macro_use]
extern crate nom;
extern crate phf;

use nom::{multispace, alpha, alphanumeric, IResult};

use std::str;
use std::fmt;
use std::io;
use std::io::Write;

pub mod rational;
pub mod value;
pub mod unit;
pub mod uval;
pub mod units;

use rational::AsFloat;

/// A mathematical expression. Can be either known or unknown (at present, all expressions are known.)
pub enum Expression {
    /// A known value (with unit).
    Value(uval::UnitValue),
    /// An error has occurred; errors propagate to all expressions in which it is involved.
    Error(value::ArithmeticError),
    /// Exponentiation, a^b
    Exp(Box<Expression>, Box<Expression>),
    /// Multiplication, a*b
    Mul(Box<Expression>, Box<Expression>),
    /// Division, a/b
    Div(Box<Expression>, Box<Expression>),
    /// Addition, a+b
    Add(Box<Expression>, Box<Expression>),
    /// Subtraction, a-b
    Sub(Box<Expression>, Box<Expression>),
    /// Negation, -a
    Neg(Box<Expression>),
    /// Function call, f(a,b,c...)
    // a Box is an owned pointer (a function is not a concrete type)
    // the function takes an f64 and returns an f64 (f64 is a double)
    // a Vec is like an ArrayList
    Call(Box<Fn(Vec<f64>) -> f64>, Vec<Expression>),
}

/// Types that can be converted to a value implement this trait.
pub trait ToValue {
    /// Convert this object to a value or return an error.
    fn to_value(&self) -> Result<uval::UnitValue, value::ArithmeticError>;
}

/// Make a Value Expression from a ToValue type
#[inline]
pub fn make_value<V: ToValue>(v: V) -> Expression {
    // Call Expression::Value on a successful result or call Expression::Error on error
    v.to_value().map(Expression::Value).unwrap_or_else(Expression::Error)
}

/// This is only called when handling user input. It treats some
/// numbers that can be handled exactly as fractions rather than
/// floating-point inexact numbers.
#[inline]
pub fn input_value(v: f64) -> Expression {
    // call the from_input method to convert rather than from_float
    make_value(uval::UnitValue::from_input(v))
}

impl ToValue for Result<uval::UnitValue, value::ArithmeticError> {
    #[inline]
    fn to_value(&self) -> Result<uval::UnitValue, value::ArithmeticError> {
        *self
    }
}

impl ToValue for uval::UnitValue {
    #[inline]
    fn to_value(&self) -> Result<uval::UnitValue, value::ArithmeticError> {
        Ok(*self)
    }
}

impl ToValue for f64 {
    #[inline]
    fn to_value(&self) -> Result<uval::UnitValue, value::ArithmeticError> {
        // this does not convert to approximate floats as rational numbers
        uval::UnitValue::from_float(*self)
    }
}

/// Expressions can be compared for equality
impl PartialEq for Expression {
    fn eq(&self, other: &Expression) -> bool {
        match (self, other) {
            (&Expression::Value(ref a), &Expression::Value(ref b)) => a == b,
            (&Expression::Exp(ref a, ref b), &Expression::Exp(ref c, ref d)) => a == c && b == d,
            (&Expression::Mul(ref a, ref b), &Expression::Mul(ref c, ref d)) => a == c && b == d,
            (&Expression::Div(ref a, ref b), &Expression::Div(ref c, ref d)) => a == c && b == d,
            (&Expression::Add(ref a, ref b), &Expression::Add(ref c, ref d)) => a == c && b == d,
            (&Expression::Sub(ref a, ref b), &Expression::Sub(ref c, ref d)) => a == c && b == d,
            (&Expression::Neg(ref a), &Expression::Neg(ref b)) => a == b,
            (&Expression::Error(ref a), &Expression::Error(ref b)) => a == b,
            // functions cannot be compared, so we assume that they're not equal.
            _ => false
        }
    }
}

/// Debug printing
impl fmt::Debug for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &Expression::Value(ref a) => write!(f, "Expression::Value({:?})", a),
            &Expression::Exp(ref a, ref b) => write!(f, "Expression::Exp({:?}, {:?})", a, b),
            &Expression::Mul(ref a, ref b) => write!(f, "Expression::Mul({:?}, {:?})", a, b),
            &Expression::Div(ref a, ref b) => write!(f, "Expression::Div({:?}, {:?})", a, b),
            &Expression::Add(ref a, ref b) => write!(f, "Expression::Add({:?}, {:?})", a, b),
            &Expression::Sub(ref a, ref b) => write!(f, "Expression::Sub({:?}, {:?})", a, b),
            &Expression::Neg(ref a) => write!(f, "Expression::Neg({:?})", a),
            &Expression::Call(_, ref a) => write!(f, "Expression::Call(fn, {:?})", a),
            &Expression::Error(ref a) => write!(f, "Expression::Error({:?})", a),
        }
    }
}

/// Display an Expression as a string (equivalent of toString())
impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            // a Value is printed as is
            &Expression::Value(ref a) => write!(f, "{}", a),
            // Error does not have a Display implementation yet
            &Expression::Error(ref a) => write!(f, "{:?}", a),
            _ => write!(f, "unknown"),
        }
    }
}

// Expression methods
impl Expression {
    /// Is this expression a known value
    #[inline]
    pub fn is_known(&self) -> bool {
        match self {
            &Expression::Value(_) => true,
            _ => false
        }
    }
    /// Is this expression an error
    #[inline]
    pub fn is_error(&self) -> bool {
        match self {
            &Expression::Error(_) => true,
            _ => false
        }
    }
    /// Extract a value or panic! (forcibly terminates the thread)
    #[inline]
    pub fn extract_value(&self) -> uval::UnitValue {
        match self {
            &Expression::Value(a) => a,
            _ => panic!("extract value of unknown")
        }
    }
    /// Extract a floating-point value or panic!
    #[inline]
    pub fn extract_float(&self) -> f64 {
        match self {
            &Expression::Value(a) => a.as_float(),
            _ => panic!("extract value of unknown")
        }
    }
}

// TODO: move functions into function module

/// Lookup a unary function by name (for convenience)
pub fn get_unary_function(res: &[u8]) -> Option<Box<Fn(f64) -> f64>> {
    match res {
        b"sin" => Some(Box::new(f64::sin)),
        b"cos" => Some(Box::new(f64::cos)),
        b"tan" => Some(Box::new(f64::tan)),
        _ => None
    }
}

/// Get a function by name (including multi-argument functions)
pub fn get_function(res: &[u8]) -> Option<Box<Fn(Vec<f64>) -> f64>> {
    // unary functions first
    if let Some(f) = get_unary_function(res) {
        return Some(Box::new(move |a: Vec<f64>| f(a[0])))
    }
    // multi-argument functions
    match res {
        b"atan2" => Some(Box::new(|a: Vec<f64>| a[0].atan2(a[1]))),
        _ => None
    }
}

/// Calculator state
pub struct Calculator {
    pub warnings: Vec<String>,
    pub result: Result<Expression, CalculatorError>,
}

/// Errors during calculation
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum CalculatorError {
    /// Caused by division by zero
    DivideByZeroError,
    /// Caused by an invalid argument
    DomainError,
    /// Caused by overflow
    OverflowError,
    /// Incompatible units or invalid use of units
    UnitError,
    /// Syntax Error
    SyntaxError,
}

impl From<value::ArithmeticError> for CalculatorError {
    fn from(e: value::ArithmeticError) -> CalculatorError {
        match e {
            value::ArithmeticError::DivideByZeroError => CalculatorError::DivideByZeroError,
            value::ArithmeticError::DomainError => CalculatorError::DomainError,
            value::ArithmeticError::OverflowError => CalculatorError::OverflowError,
            value::ArithmeticError::UnitError => CalculatorError::UnitError,
        }
    }
}

/// Replacement for recognize! since it doesn't work with methods
#[doc(hidden)]
macro_rules! recognize2 (
    ($i:expr, $submac:ident!( $($args:tt)* )) => (
        {
            use nom::HexDisplay;
            match $submac!($i, $($args)*) {
                nom::IResult::Done(i,_)     => {
                    let index = ($i).offset(i);
                    nom::IResult::Done(i, &($i)[..index])
                },
                nom::IResult::Error(e)      => nom::IResult::Error(e),
                nom::IResult::Incomplete(i) => nom::IResult::Incomplete(i)
            }
        }
    );
    ($i:expr, $f:expr) => (
        recognize!($i, call!($f))
    );
);

/// Replacement for error!
#[doc(hidden)]
macro_rules! error2 (
  ($i:expr, $self_:ident, $code:expr, $submac:ident!( $($args:tt)* )) => (
    {
      let cl = || {
        $submac!($i, $($args)*)
      };

      match cl() {
        nom::IResult::Incomplete(x) => nom::IResult::Incomplete(x),
        nom::IResult::Done(i, o)    => nom::IResult::Done(i, o),
        nom::IResult::Error(e)      => {
          return ($self_, nom::IResult::Error(nom::Err::NodePosition($code, $i, Box::new(e))))
        }
      }
    }
  );
);

impl Calculator {
    fn new() -> Calculator {
        Calculator {
            warnings: Vec::new(),
            result: Err(CalculatorError::SyntaxError),
        }
    }

    fn calculate(input: &[u8]) -> Calculator {
        let mut s = str::from_utf8(input).unwrap().to_owned();
        Calculator::run(&mut s)
    }

    fn run(input: &mut String) -> Calculator {
        let mut calc = Calculator::new();
        input.push('?');
        calc.result = match calc.input(input.as_bytes()) {
            (_, IResult::Done(_, val)) => match &val {
                    &Expression::Error(a) => Err(From::from(a)),
                    _ => Ok(val),
                },
            _ => Err(CalculatorError::SyntaxError),
        };
        calc
    }

    /// A parenthetical expression
    method!(pub parens<Calculator, Expression>, self, alt!(
        // either an expression in parentheses
            delimited!(char!('(')
          , preceded!(opt!(multispace), call_m!(self.expr))
          , preceded!(opt!(multispace), char!(')')))
        // or a function name followed by parentheses and comma-separated arguments
          | chain!(
              func: map_opt!(alphanumeric, get_function)
            ~ args: delimited!(char!('('), preceded!(opt!(multispace), separated_nonempty_list!(delimited!(opt!(multispace), char!(','), opt!(multispace)), call_m!(self.expr))), preceded!(opt!(multispace), char!(')'))),
              || self.simplify1(Expression::Call(func, args))
          )));

    /// Recognize integers and numbers with digits on the left side of decimal point (e.g. 57, 2.3)
    #[inline]
    method!(recognize_number1<Calculator, &[u8]>, self, recognize2!(
            chain!(call!(Calculator::decimal)
                 ~ preceded!(char!('.'), opt!(call!(Calculator::decimal)))?
                 ~ preceded!(one_of!("eE"),
                       preceded!(opt!(one_of!("+-")), call!(Calculator::decimal)))?,
                 || ())));
    /// Recognize numbers with a decimal point followed by digits (e.g. .2, .7)
    #[inline]
    method!(recognize_number2<Calculator, &[u8]>, self, recognize2!(
            chain!(char!('.')
                 ~ call!(Calculator::decimal)
                 ~ preceded!(one_of!("eE"),
                       preceded!(opt!(one_of!("+-")), call!(Calculator::decimal)))?,
                 || ())));
    /// Convert a [u8] (char array) to a String
    #[inline]
    fn stringify_u8(res: &[u8]) -> Result<String, str::Utf8Error> {
        Ok(try!(str::from_utf8(res)).to_owned())
    }
    /// Convert a [u8] to a String and add a 0 at the front (.2 -> 0.2)
    #[inline]
    fn prepend_zero(res: &[u8]) -> Result<String, str::Utf8Error> {
        let mut s = try!(str::from_utf8(res)).to_owned();
        s.insert(0, '0');
        Ok(s)
    }

    /// A decimal value (including underscores); underscores are removed
    /// An underscore can be used to provide clarity, e.g. 1_200 for 1,200
    #[inline]
    named!(decimal<()>, value!((), many1!(one_of!("0123456789_"))));

    /// A number is one of the two number forms above
    method!(pub number<Calculator, f64>, self, map_res!(map_res!(
                alt!(call_m!(self.recognize_number1) => {Calculator::stringify_u8}
                   | call_m!(self.recognize_number2) => {Calculator::prepend_zero}),
                // Remove underscores
                |a: Result<String, str::Utf8Error>|
                    Ok(try!(a).replace('_', ""))
                    as Result<String, str::Utf8Error>),
                // then interpret as a float
                |a: String| a.parse()));

    /// Look up a numerical constant (unitless)
    pub fn get_numerical_constant(&self, res: &[u8]) -> Option<f64> {
        match &res {
            &b"e" => Some(std::f64::consts::E),
            &b"pi" => Some(std::f64::consts::PI),
            _ => None
        }
    }

    /// Look up a united value
    pub fn get_unit(&self, res: &[u8]) -> Option<uval::UnitValue> {
        match str::from_utf8(res) {
            Ok(a) => units::get(a),
            Err(_) => None,
        }
    }

    /// A numerical constant consists of only letters
    #[inline]
    method!(pub num_const<Calculator, f64>, self, map_opt!(alpha, |x| self.get_numerical_constant(x)));
    /// A united constant may contains numbers and underscores
    #[inline]
    method!(pub unit_const<Calculator, uval::UnitValue>, self, map_opt!(recognize2!(many1!(one_of!("0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_"))), |x| self.get_unit(x)));

    /// The innermost level is either parentheticals, numbers, or constants
    method!(pub atom<Calculator, Expression>, self, alt!(call_m!(self.parens)
                                                      | call_m!(self.number) => {input_value}
                                                      | call_m!(self.num_const) => {make_value}
                                                      | call_m!(self.unit_const) => {Expression::Value}));

    /// Implied multiplication without spaces has the highest precedence
    // e.g. 1/2pi => 1/(2pi), but 1/2 pi => pi/2
    method!(pub imul<Calculator, Expression>, self, chain!(
           first: call_m!(self.atom)
         ~ others: many0!(call_m!(self.atom)), ||
        others.into_iter().fold(first,
            |lhs, rhs| self.simplify1(
                         Expression::Mul(Box::new(lhs), Box::new(rhs))))
    ));

    /// A unary value such as + and -.
    method!(pub unary<Calculator, Expression>, self, alt!(call_m!(self.exp)
                                 | chain!(op: chain!(
                                         o: alt!(char!('+') | char!('-'))
                                       ~ multispace?, || o)
                                 ~ val: call_m!(self.unary), ||{
        match op {
            '+' => val,
            '-' => self.simplify1(Expression::Neg(Box::new(val))),
            _ => val,
        }
    })));

    /// Exponentiation (right associative)
    method!(pub exp<Calculator, Expression>, self, chain!(
           lhs: call_m!(self.imul)
         ~ rhs: preceded!(preceded!(opt!(multispace), char!('^')),
                          preceded!(opt!(multispace), call_m!(self.unary)))?, ||
        match (lhs, rhs) {
            (lhs, None) => lhs,
            (lhs, Some(b))
                => self.simplify1(Expression::Exp(Box::new(lhs), Box::new(b))),
        }
    ));

    /// A single factor-term with * or / (or whitespace, which is treated as multiplication)
    method!(pub facterm<Calculator, (char, Expression)>, self,
            tuple!(alt!(
                   preceded!(opt!(multispace), char!('*'))
                 | preceded!(opt!(multispace), char!('/'))
                 | value!('*',
                          preceded!(multispace,
                                    error2!(self, nom::ErrorKind::NoneOf,
                                           peek!(none_of!("+-")))))),
                   preceded!(opt!(multispace), call_m!(self.unary))));

    /// A thing followed by things with operators
    method!(pub fac<Calculator, Expression>, self,
            chain!(first: call_m!(self.unary)
                 ~ others: many0!(call_m!(self.facterm)), ||
        others.into_iter().fold(first, |lhs, (op, rhs)| self.simplify1(
                match op {
                    '*' => Expression::Mul(Box::new(lhs), Box::new(rhs)),
                    '/' => Expression::Div(Box::new(lhs), Box::new(rhs)),
                    _   => Expression::Mul(Box::new(lhs), Box::new(rhs))
                }))
    ));

    /// An expression consists of one factor followed by more terms preceded by + or -.
    method!(pub expr<Calculator, Expression>, self,
            chain!(first: call_m!(self.fac)
                 ~ others: many0!(tuple!(
                           preceded!(opt!(multispace),
                               alt!(char!('+') | char!('-'))),
                               preceded!(opt!(multispace), call_m!(self.fac)))), ||
        others.into_iter().fold(first, |lhs, (op, rhs)| self.simplify1(
                match op {
                    '+' => Expression::Add(Box::new(lhs), Box::new(rhs)),
                    '-' => Expression::Sub(Box::new(lhs), Box::new(rhs)),
                    _   => Expression::Add(Box::new(lhs), Box::new(rhs))
                }))
    ));

    /// User input has a ? appended so that it does not try to match things after the input (nom yields an Incomplete)
    method!(pub input<Calculator, Expression>, self, chain!(opt!(multispace) ~ res: call_m!(self.expr) ~ opt!(multispace) ~ char!('?'), ||{res}));

    /// Simplify 1 part of an expression
    fn simplify1(&self, expr: Expression) -> Expression {
        /// All values in an array are known
        fn all_known(a: &Vec<Expression>) -> bool {
            a.iter().all(Expression::is_known)
        }
        /// Some value is an error, so we should return an error
        fn any_error(a: &Vec<Expression>) -> bool {
            a.iter().any(Expression::is_error)
        }
        /// Make it more readable by renaming types
        use Expression as E;
        use Expression::Value as V;
        match expr {
            E::Exp(box V(ref a), box V(ref b)) => make_value(a.pow(b)),
            E::Exp(_, box e @ E::Error(_)) => e,
            E::Exp(box e @ E::Error(_), _) => e,
            E::Mul(box V(ref a), box V(ref b)) => make_value(a.mul(b)),
            E::Mul(_, box e @ E::Error(_)) => e,
            E::Mul(box e @ E::Error(_), _) => e,
            E::Div(box V(ref a), box V(ref b)) => make_value(a.div(b)),
            E::Div(_, box e @ E::Error(_)) => e,
            E::Div(box e @ E::Error(_), _) => e,
            E::Add(box V(ref a), box V(ref b)) => make_value(a.add(b)),
            E::Add(_, box e @ E::Error(_)) => e,
            E::Add(box e @ E::Error(_), _) => e,
            E::Sub(box V(ref a), box V(ref b)) => make_value(a.sub(b)),
            E::Sub(_, box e @ E::Error(_)) => e,
            E::Sub(box e @ E::Error(_), _) => e,
            E::Neg(box V(a)) => make_value(-a),
            E::Neg(box E::Neg(box a)) => a,
            E::Neg(box e @ E::Error(_)) => e,
            /// Call a function by extracting the floating-point values of the arguments
            E::Call(ref f, ref a) if all_known(a) => make_value(f(a.iter().map(Expression::extract_float).collect())),
            /// Forward the first error
            E::Call(_, ref a) if any_error(a) => match a.iter().find(|e| e.is_error()).expect("no error found") {
                &E::Error(a) => E::Error(a),
                _ => panic!("not actually an error")
            },
            expr => expr
        }
    }

}
// the following tests are self-explanatory.
#[cfg(test)]
mod tests {
    use super::*;
    use nom::*;
    use std;
    use rational::AsFloat;
    /// Macro used for testing an expression against a known value
    macro_rules! test_expr {
        ($x:expr, $v: expr) => (assert_eq!(Calculator::calculate($x.as_bytes()).result, Ok(make_value($v))));
    }
    /// Macro used for approximately equal
    macro_rules! test_approx {
        ( $x:expr, $v: expr) => ({
            let res = Calculator::calculate($x.as_bytes()).result;
            match &res {
                &Ok(Expression::Value(val)) => {
                    assert!((val.as_float() - $v).abs() < 1e-6)
                },
                _ => panic!("input not consumed: {:?}", res)
            }});
    }
    /// An expression should not parse correctly.
    macro_rules! fail_expr {
        ( $x: expr ) => (match Calculator::calculate($x.as_bytes()).result { Ok(_) => panic!("should have failed"), _ => () })
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

    #[test]
    fn test_thomas() {
        test_expr!("1+1", 2.0);
        test_expr!("2^(3*2-4)-4", 0.0);
    }

    #[test]
    fn test_floating() {
        test_expr!("5", 5.0);
        test_expr!("2.3e2", 230.0);
        test_expr!("5e-2", 0.05);
        test_expr!("8_230_999", 8_230_999.0);
        fail_expr!("_");
        test_expr!(".2", 0.2);
        // Rust reference examples
        test_expr!("123.0", 123.0f64);
        test_expr!("0.1", 0.1f64);
        test_expr!("12E+99", 12E+99_f64);
        test_expr!("2.", 2.);
    }

    #[test]
    fn test_num_const() {
        test_expr!("pi", std::f64::consts::PI);
        test_expr!("e", std::f64::consts::E);
    }

    #[test]
    fn test_function() {
        test_approx!("sin(pi/6)", 0.5);
        test_approx!("atan2(1, 1)", std::f64::consts::FRAC_PI_4);
    }
}

/// Main function; we read until we find "quit"
pub fn main() {
    println!("Welcome to Unit Calculator v1.0.0 by James Dong.");
    println!("see src/units.rs for a list of units.");
    println!("type \"quit\" to quit.");
    println!("");
    // REPL
    loop {
        let mut line = String::new();
        print!("ucalc> ");
        io::stdout().flush().expect("error flushing");
        io::stdin().read_line(&mut line).expect("error reading");
        if line.trim() == "quit" { break }
        // TODO: move to separate function
        // add a question mark to end the end of the input
        let calc = Calculator::run(&mut line);
        match calc.result {
            Ok(val) => {
                for warn in calc.warnings {
                    println!("{}", warn);
                }
                println!("=> {}", val)
            },
            Err(err) => println!("error: {:?}", err),
        }
    }
}
