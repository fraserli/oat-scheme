use std::fmt::Display;
use std::rc::Rc;

use crate::environment::Environment;
use crate::error::{Error, Result};
use crate::unscheme;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Void,

    Symbol(String),
    Number(f64),
    String(String),
    Character(char),
    Boolean(bool),

    EmptyList,
    Pair((Rc<Value>, Rc<Value>)),

    Procedure(Procedure),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Procedure {
    Primitive(PrimitiveProcedure),
    Compound {
        parameters: Vec<String>,
        body: Vec<Rc<Value>>,
        captures: Vec<(String, Rc<Value>)>,
    },
}

pub type PrimitiveProcedure = fn(&Rc<Value>, &mut Environment) -> Result<Rc<Value>>;

impl Value {
    pub fn void() -> Rc<Self> {
        Rc::new(Self::Void)
    }

    pub fn symbol(s: &str) -> Rc<Self> {
        Rc::new(Self::Symbol(s.to_owned()))
    }

    pub fn number(n: f64) -> Rc<Self> {
        Rc::new(Self::Number(n))
    }

    pub fn string(s: &str) -> Rc<Self> {
        Rc::new(Self::String(s.to_owned()))
    }

    pub fn character(c: char) -> Rc<Self> {
        Rc::new(Self::Character(c))
    }

    pub fn boolean(b: bool) -> Rc<Self> {
        Rc::new(Self::Boolean(b))
    }

    pub fn empty_list() -> Rc<Self> {
        Rc::new(Self::EmptyList)
    }

    pub fn pair(l: &Rc<Self>, r: &Rc<Self>) -> Rc<Self> {
        Rc::new(Self::Pair((Rc::clone(l), Rc::clone(r))))
    }

    pub fn procedure(p: Procedure) -> Rc<Self> {
        Rc::new(Self::Procedure(p))
    }

    pub fn to_bool(&self) -> bool {
        !matches!(self, Self::Boolean(false))
    }
}

impl Iterator for &Value {
    type Item = Result<Rc<Value>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Value::EmptyList => None,
            Value::Pair((car, cdr)) => {
                *self = cdr;
                Some(Ok(Rc::clone(car)))
            }
            _ => {
                *self = &Value::EmptyList;
                Some(Err(Error::ExpectedList(Rc::new(self.clone()))))
            }
        }
    }
}

impl FromIterator<Rc<Value>> for Value {
    fn from_iter<T: IntoIterator<Item = Rc<Value>>>(iter: T) -> Self {
        let mut elems: Vec<_> = iter.into_iter().collect();
        elems.reverse();
        elems
            .into_iter()
            .fold(Value::EmptyList, |acc, e| Value::Pair((e, Rc::new(acc))))
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
            Self::Procedure { .. } => f.write_str("#<procedure>"),
            Self::Pair((car, cdr)) => {
                if let (Ok(quote), Ok(value)) = (unscheme!(car => Symbol), unscheme!(cdr => [any]))
                    && quote == "quote"
                {
                    return write!(f, "'{value}");
                }

                write!(f, "({car}")?;

                let mut curr = Rc::clone(cdr);

                loop {
                    match *curr {
                        Self::Pair((ref car, ref cdr)) => {
                            write!(f, " {car}")?;
                            curr = Rc::clone(cdr);
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
