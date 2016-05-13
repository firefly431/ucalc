use rational::*;

use std::cmp;
use std::cmp::Ord;
use std::ops::{Add,Sub,Mul,Div};
use std::fmt;

#[derive(Copy, Clone, Debug)]
pub enum Value {
    Inexact(f64),
    Exact(Rational),
}

impl AsFloat for Value {
    #[inline]
    fn as_float(&self) -> f64 {
        match self {
            &Value::Inexact(a) => a,
            &Value::Exact(ref a) => a.as_float(),
        }
    }
}

impl PartialEq for Value {
    #[inline]
    fn eq(&self, other: &Value) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
    #[inline]
    fn ne(&self, other: &Value) -> bool {
        self.cmp(other) != cmp::Ordering::Equal
    }
}

impl Eq for Value {}

impl Ord for Value {
    fn cmp(&self, other: &Value) -> cmp::Ordering {
        match (self, other) {
            (&Value::Inexact(ref a), &Value::Inexact(ref b)) => a.partial_cmp(b).unwrap(),
            (&Value::Exact(ref a), &Value::Exact(ref b)) => a.cmp(b),
            (a, b) => a.as_float().partial_cmp(&b.as_float()).unwrap(),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Value) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// includes unit errors
#[derive(Debug, PartialEq, Eq, Hash)]
enum ArithmeticError {
    DivideByZeroError,
    DomainError,
    OverflowError,
}

impl Value {
    fn from_float(f: f64) -> Result<Value, ArithmeticError> {
        if !f.is_nan() {
            if (f * 8.0).fract() != 0.0 {
                Ok(Value::Inexact(f))
            } else {
                let num = f * 8.0;
                if num.abs() > i32::max_value() as f64 {
                    Ok(Value::Inexact(f))
                } else {
                    Rational::new(num as i32, 8).or(Err(ArithmeticError::DomainError)).map(Value::Exact)
                }
            }
        } else {
            Err(ArithmeticError::DomainError)
        }
    }
    #[inline]
    fn from_inexact(f: f64) -> Result<Value, ArithmeticError> {
        if !f.is_nan() {
            Ok(Value::Inexact(f))
        } else {
            Err(ArithmeticError::DomainError)
        }
    }
    #[inline]
    fn get_exact(&self) -> Option<&Rational> {
        match self {
            &Value::Exact(ref a) => Some(a),
            &Value::Inexact(_) => None,
        }
    }
    #[inline]
    fn as_integer(&self) -> Option<i32> {
        match self {
            &Value::Exact(ref a) => if a.is_integer() { Some(a.num) } else { None },
            &Value::Inexact(a) => if a.fract() == 0.0 && a.abs() <= i32::max_value() as f64 { Some(a as i32) } else { None },
        }
    }
    fn add(&self, other: &Value) -> Result<Value, ArithmeticError> {
        match (self.get_exact(), other.get_exact()) {
            (Some(a), Some(b)) => a.add(b).map(Value::Exact).or_else(|_| Value::from_inexact(self.as_float() + other.as_float())),
            _ => Value::from_inexact(self.as_float() + other.as_float())
        }
    }
    fn sub(&self, other: &Value) -> Result<Value, ArithmeticError> {
        match (self.get_exact(), other.get_exact()) {
            (Some(a), Some(b)) => a.sub(b).map(Value::Exact).or_else(|_| Value::from_inexact(self.as_float() - other.as_float())),
            _ => Value::from_inexact(self.as_float() - other.as_float())
        }
    }
    fn mul(&self, other: &Value) -> Result<Value, ArithmeticError> {
        match (self.get_exact(), other.get_exact()) {
            (Some(a), Some(b)) => a.mul(b).map(Value::Exact).or_else(|_| Value::from_inexact(self.as_float() * other.as_float())),
            _ => Value::from_inexact(self.as_float() * other.as_float())
        }
    }
    fn div(&self, other: &Value) -> Result<Value, ArithmeticError> {
        match (self.get_exact(), other.get_exact()) {
            (Some(a), Some(b)) => a.div(b).map(Value::Exact).or_else(|_| Value::from_inexact(self.as_float() / other.as_float())),
            _ => Value::from_inexact(self.as_float() / other.as_float())
        }
    }
    fn pow(&self, other: &Value) -> Result<Value, ArithmeticError> {
        match self.get_exact() {
            Some(a) => if let Some(e) = other.as_integer() { a.pow(e).map(Value::Exact).or_else(|_| Value::from_inexact(a.as_float().powi(e))) } else { Value::from_inexact(a.as_float().powf(other.as_float())) },
            None => Value::from_inexact(self.as_float().powf(other.as_float()))
        }
    }
}

impl Add for Value {
    type Output = Value;
    fn add(self, other: Value) -> Value {
        (&self).add(&other).unwrap()
    }
}

impl Sub for Value {
    type Output = Value;
    fn sub(self, other: Value) -> Value {
        (&self).sub(&other).unwrap()
    }
}

impl Mul for Value {
    type Output = Value;
    fn mul(self, other: Value) -> Value {
        (&self).mul(&other).unwrap()
    }
}

impl Div for Value {
    type Output = Value;
    fn div(self, other: Value) -> Value {
        (&self).div(&other).unwrap()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &Value::Inexact(a) => write!(f, "{}", a),
            &Value::Exact(ref a) => write!(f, "{}", a),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rational::*;
    use std::f64;
    use std::cmp;
    use std::cmp::Ordering;
    use std::fmt;
    use std::fmt::{Write, Display};

    #[test]
    fn test_as_float() {
        for a in vec![f64::INFINITY, -f64::INFINITY, 0.0, -0.0, 8.0, 0.125, 10e100] {
            assert_eq!(Value::from_float(a).unwrap().as_float(), a);
        }
    }

    macro_rules! val {
        ( V $a:expr ) => ( Value::from_float($a).unwrap() )
    }

    #[test]
    fn test_simple_arithmetic() {
        assert_eq!(val!(V 4.0) + val!(V 1.0), val!(V 5.0));
        assert_eq!(val!(V 4.0) - val!(V 1.0), val!(V 3.0));
        assert_eq!(val!(V 4.0) * val!(V 1.0), val!(V 4.0));
        assert_eq!(val!(V 4.0) / val!(V 2.0), val!(V 2.0));
    }
}
