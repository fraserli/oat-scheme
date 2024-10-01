use std::fmt::Display;

use gc::{Finalize, Gc, Trace};

use crate::environment::Environment;
use crate::error::{Error, Result};
use crate::unscheme;

#[derive(Debug, Clone, PartialEq, Trace, Finalize)]
pub enum Value {
    Void,

    Symbol(String),
    Number(f64),
    String(String),
    Character(char),
    Boolean(bool),

    EmptyList,
    Pair((Gc<Value>, Gc<Value>)),

    PrimitiveProcedure(PrimitiveProcedure),
    Procedure(Procedure),
}

#[derive(Debug, Clone, PartialEq, Trace, Finalize)]
pub struct PrimitiveProcedure(
    #[unsafe_ignore_trace] pub fn(&Gc<Value>, &mut Environment) -> Result<Gc<Value>>,
);

#[derive(Debug, Clone, PartialEq, Trace, Finalize)]
pub struct Procedure {
    pub parameters: Vec<String>,
    pub body: Vec<Gc<Value>>,
    pub captures: Vec<(String, Gc<Value>)>,
}

impl Value {
    pub fn void() -> Gc<Self> {
        Gc::new(Self::Void)
    }

    pub fn symbol(s: &str) -> Gc<Self> {
        Gc::new(Self::Symbol(s.to_owned()))
    }

    pub fn number(n: f64) -> Gc<Self> {
        Gc::new(Self::Number(n))
    }

    pub fn string(s: &str) -> Gc<Self> {
        Gc::new(Self::String(s.to_owned()))
    }

    pub fn character(c: char) -> Gc<Self> {
        Gc::new(Self::Character(c))
    }

    pub fn boolean(b: bool) -> Gc<Self> {
        Gc::new(Self::Boolean(b))
    }

    pub fn empty_list() -> Gc<Self> {
        Gc::new(Self::EmptyList)
    }

    pub fn pair(l: &Gc<Self>, r: &Gc<Self>) -> Gc<Self> {
        Gc::new(Self::Pair((l.clone(), r.clone())))
    }

    pub fn procedure(
        parameters: Vec<String>,
        body: Vec<Gc<Value>>,
        captures: Vec<(String, Gc<Value>)>,
    ) -> Gc<Self> {
        Gc::new(Self::Procedure(Procedure {
            parameters,
            body,
            captures,
        }))
    }

    pub fn to_bool(&self) -> bool {
        !matches!(self, Self::Boolean(false))
    }
}

impl Iterator for &Value {
    type Item = Result<Gc<Value>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Value::EmptyList => None,
            Value::Pair((car, cdr)) => {
                *self = cdr;
                Some(Ok(car.clone()))
            }
            _ => {
                const EMPTY_LIST: &Value = &Value::EmptyList;
                *self = EMPTY_LIST;
                Some(Err(Error::ExpectedList(Gc::new(self.clone()))))
            }
        }
    }
}

impl FromIterator<Gc<Value>> for Value {
    fn from_iter<T: IntoIterator<Item = Gc<Value>>>(iter: T) -> Self {
        let mut elems: Vec<_> = iter.into_iter().collect();
        elems.reverse();
        elems
            .into_iter()
            .fold(Value::EmptyList, |acc, e| Value::Pair((e, Gc::new(acc))))
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Void => f.write_str("#<void>"),
            Self::Symbol(name) => {
                if name.chars().any(char::is_whitespace) {
                    write!(f, "|{name}|")
                } else {
                    f.write_str(name)
                }
            }
            Self::Number(n) => f.write_str(&n.to_string()),
            Self::String(s) => write!(f, "\"{s}\""),
            Self::Character(c) => write!(f, "#\\{c}"),
            Self::Boolean(b) => f.write_str(match b {
                true => "#t",
                false => "#f",
            }),
            Self::EmptyList => f.write_str("()"),
            Self::PrimitiveProcedure(_) => f.write_str("#<procedure>"),
            Self::Procedure(_) => f.write_str("#<procedure>"),
            Self::Pair((car, cdr)) => {
                if let (Ok(quote), Ok(value)) = (unscheme!(car => Symbol), unscheme!(cdr => [any]))
                    && quote == "quote"
                {
                    return write!(f, "'{value}");
                }

                write!(f, "({car}")?;

                let mut curr = cdr.clone();

                loop {
                    match *curr {
                        Self::Pair((ref car, ref cdr)) => {
                            write!(f, " {car}")?;
                            curr = cdr.clone();
                        }
                        Self::EmptyList => {
                            f.write_str(")")?;
                            break;
                        }
                        _ => {
                            write!(f, " . {curr})")?;
                            break;
                        }
                    }
                }

                Ok(())
            }
        }
    }
}
