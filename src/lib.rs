#![feature(let_chains)]

mod builtin;
mod environment;
mod error;
mod eval;
mod parse;
mod value;

pub use environment::Environment;
pub use error::Error;
pub use eval::{eval, eval_to_value};
pub use gc::{self, Gc};
pub use parse::{parse, parse_one};
pub use value::Value;

/// Constructs an Gc<[Value](Value)> using S-expression syntax.
///
/// ```
/// use oat_scheme::{Gc, parse_one, scheme, Value};
///
/// // Create a single value
/// assert_eq!(scheme!({ true }), parse_one("#t").unwrap());
/// assert_eq!(scheme!({ 123 }), parse_one("123").unwrap());
/// assert_eq!(scheme!({ "hello" }), parse_one("\"hello\"").unwrap());
/// assert_eq!(scheme!({ () }), parse_one("()").unwrap());
/// assert_eq!(scheme!({ hello }), parse_one("hello").unwrap());
/// assert_eq!(scheme!({ [hello world] }), parse_one("|hello world|").unwrap());
///
/// // Create a list
/// assert_eq!(scheme!(), parse_one("()").unwrap());
/// assert_eq!(scheme!(a "a" 1 false), parse_one("(a \"a\" 1 #f)").unwrap());
/// assert_eq!(
///     scheme!(define (double x) ([+] x x)),
///     parse_one("(define (double x) (+ x x))").unwrap(),
/// );
///
/// // Insert an existing value
/// let value: Gc<Value> = scheme!(cons a b);
/// assert_eq!(
///     scheme!(1 2 [[&value]] 3),
///     parse_one("(1 2 (cons a b) 3)").unwrap(),
/// );
/// ```
#[macro_export]
macro_rules! scheme {
    ({ () }) => { $crate::Value::empty_list() };
    ({ ($($xs:tt)+) }) => { scheme!($($xs)+) };
    ({ true }) => { $crate::Value::boolean(true) };
    ({ false }) => { $crate::Value::boolean(false) };
    ({ $value:ident }) => { $crate::Value::symbol(stringify!($value)) };
    ({ [[$value:expr]] }) => { $value.clone() };
    ({ [$($value:tt)+] }) => { $crate::Value::symbol(stringify!($($value)+)) };
    ({ $value:literal }) => {
        if let Some(n) = (&$value as &dyn std::any::Any).downcast_ref::<i32>() {
            $crate::Value::number(*n as f64)
        } else if let Some(n) = (&$value as &dyn std::any::Any).downcast_ref::<f64>() {
            $crate::Value::number(*n)
        } else if let Some(s) = (&$value as &dyn std::any::Any).downcast_ref::<&str>() {
            $crate::Value::string(s)
        } else {
            panic!("invalid scheme literal");
        }
    };

    () => { $crate::Value::empty_list() };
    ($value:tt) => {
        $crate::Value::pair(&scheme!({ $value }), &$crate::Value::empty_list())
    };
    ($value:tt $($rest:tt)+) => {
        $crate::Value::pair(&scheme!({ $value }), &scheme!( $($rest)+ ))
    };

}

#[macro_export]
macro_rules! unscheme {
    ($value:expr => any) => { Ok($value.clone()) };
    ($value:expr => [rest]) => { Ok($value.clone()) };
    ($value:expr => $variant:ident) => {
        match &**$value {
            $crate::Value::$variant(inner) => Ok(inner.clone()),
            _ => Err($crate::Error::TypeMismatch(
                stringify!($variant).to_string().to_lowercase(),
                $value.clone()
            )),
        }
    };
    ($value:expr => [$variant:ident]) => {
        unscheme!($value => Pair).and_then(|(car, cdr)| match *cdr {
            $crate::Value::EmptyList => unscheme!(&car => $variant),
            _ => Err($crate::Error::ExpectedList(cdr.clone())),
        })
    };
    ($value:expr => [$variant:ident, $($rest:ident),*]) => {
        unscheme!($value => Pair)
            .map_err(|_| $crate::Error::ExpectedList($value.clone()))
            .and_then(|(car, cdr)| Ok((
                unscheme!(&car => $variant)?,
                unscheme!(&cdr => [$($rest),*])?,
            )))
    };

    // Evaluate before destructuring
    ($value:expr, $env:ident ==> any) => { $crate::eval_to_value($value.clone(), $env) };
    ($value:expr, $env:ident ==> [rest]) => { Ok($value.clone()) };
    ($value:expr, $env:ident ==> $variant:ident) => {
        $crate::eval_to_value($value.clone(), $env)
            .and_then(|value| unscheme!(&value => $variant))
    };
    ($value:expr, $env:ident ==> [$variant:ident]) => {
        unscheme!($value => [any])
            .and_then(|value| $crate::eval_to_value(value, $env))
            .and_then(|value| unscheme!(&value => $variant))
    };
    ($value:expr, $env:ident ==> [$variant:ident, $($rest:ident),*]) => {
        unscheme!($value => [any, rest]).and_then(|(car, cdr)| Ok((
            unscheme!(&car, $env ==> $variant)?,
            unscheme!(&cdr, $env ==> [$($rest),*])?,
        )))
    };
}
