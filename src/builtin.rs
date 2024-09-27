use std::rc::Rc;

use crate::error::Error;
use crate::unscheme;
use crate::value::{PrimitiveProcedure, Procedure, Value};

pub fn builtins() -> impl Iterator<Item = (String, Rc<Value>)> {
    BUILTINS
        .iter()
        .map(|&(s, f)| (s.to_owned(), Value::procedure(Procedure::Primitive(f))))
}

const BUILTINS: &[(&str, PrimitiveProcedure)] = &[
    ("display", |params, env| {
        let value = unscheme!(params, env ==> [any])?;
        match *value {
            Value::String(ref s) => println!("{}", s),
            Value::Character(c) => println!("{}", c),
            _ => println!("{}", value),
        }
        Ok(Value::void())
    }),
    ("not", |params, env| {
        let b = unscheme!(params, env ==> [Boolean])?;
        Ok(Value::boolean(!b))
    }),
    ("eq?", |params, env| {
        let (lhs, rhs) = unscheme!(params, env ==> [any, any])?;
        Ok(Value::boolean(eq(&lhs, &rhs)))
    }),
    ("cons", |params, env| {
        let (car, cdr) = unscheme!(params, env ==> [any, any])?;
        Ok(Value::pair(&car, &cdr))
    }),
    ("car", |params, env| {
        let (car, _cdr) = unscheme!(params, env ==> [Pair])?;
        Ok(car)
    }),
    ("cdr", |params, env| {
        let (_car, cdr) = unscheme!(params, env ==> [Pair])?;
        Ok(cdr)
    }),
    ("list", |params, env| {
        Ok(Rc::new(
            params
                .as_ref()
                .map(|p| p?.eval_to_value(env))
                .collect::<Result<_, _>>()?,
        ))
    }),
    ("abs", |params, env| {
        Ok(Value::number(unscheme!(params, env ==> [Number])?.abs()))
    }),
    ("+", |params, env| {
        let numbers = params.map(|p| unscheme!(&p?, env ==> Number));
        Ok(Value::number(numbers.sum::<Result<_, _>>()?))
    }),
    ("-", |params, env| match params.count() {
        0 => Err(Error::IncorrectArity(1, 0)),
        1 => Ok(Value::number(-unscheme!(params, env ==> [Number])?)),
        _ => {
            let (minuend, rest) = unscheme!(params, env ==> [Number, rest])?;
            let subtrahend = rest
                .as_ref()
                .try_fold(0.0, |acc, p| Ok(acc + unscheme!(&p?, env ==> Number)?))?;
            Ok(Value::number(minuend - subtrahend))
        }
    }),
    ("*", |params, env| {
        let numbers = params.map(|p| unscheme!(&p?, env ==> Number));
        Ok(Value::number(numbers.product::<Result<_, _>>()?))
    }),
    ("/", |params, env| match params.count() {
        0 => Err(Error::IncorrectArity(1, 0)),
        1 => Ok(Value::number(1.0 / unscheme!(params, env ==> [Number])?)),
        _ => {
            let (dividend, rest) = unscheme!(params, env ==> [Number, rest])?;
            let divisor = rest
                .as_ref()
                .try_fold(1.0, |acc, p| Ok(acc * unscheme!(&p?, env ==> Number)?))?;
            Ok(Value::number(dividend / divisor))
        }
    }),
    ("string-length", |params, env| {
        let string = unscheme!(params, env ==> [String])?;
        Ok(Value::number(string.chars().count() as f64))
    }),
    ("string-ref", |params, env| {
        let (string, idx) = unscheme!(params, env ==> [String, Number])?;
        let c = string.chars().nth(idx as usize);
        Ok(Value::character(
            c.ok_or(Error::IndexOutOfBounds(idx as usize))?,
        ))
    }),
    ("substring", |params, env| {
        let (string, (start, end)) = unscheme!(params, env ==> [String, Number, Number])?;
        Ok(Value::string(&string[start as usize..end as usize]))
    }),
    ("string-append", |params, env| {
        let string = params
            .map(|p| unscheme!(&p?, env ==> String))
            .collect::<Result<String, _>>()?;
        Ok(Rc::new(Value::String(string)))
    }),
];

fn eq(lhs: &Value, rhs: &Value) -> bool {
    match (lhs, rhs) {
        (Value::Symbol(ref l), Value::Symbol(ref r)) => l == r,
        (Value::Number(ref l), Value::Number(ref r)) => l == r,
        (Value::String(ref l), Value::String(ref r)) => l == r,
        (Value::Character(ref l), Value::Character(ref r)) => l == r,
        (Value::Boolean(ref l), Value::Boolean(ref r)) => l == r,
        (Value::EmptyList, Value::EmptyList) => true,
        (Value::Pair(_), Value::Pair(_)) => lhs.zip(rhs).all(|(l, r)| match l {
            Ok(l) => r.is_ok_and(|r| eq(&l, &r)),
            Err(_) => r.is_err(),
        }),
        _ => false,
    }
}
