use std::rc::Rc;

use crate::value::Value;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum Error {
    UndefinedVariable(String),
    EmptyApplication,
    EmptyProcedure,
    ExpectedProcedure(Rc<Value>),
    ExpectedValue(Rc<Value>),
    IncorrectArity(usize, usize),
    TypeMismatch(String, Rc<Value>),
    ExpectedList(Rc<Value>),
    IndexOutOfBounds(usize),
    UnexpectedEndOfInput,
    ParseError(Vec<(chumsky::span::SimpleSpan, Vec<String>, char)>),
}

impl<'a> Error {
    pub fn from_parse_errors(parse_errors: Vec<chumsky::error::Rich<'a, char>>) -> Self {
        let mut errors = Vec::with_capacity(parse_errors.len());

        for error in parse_errors.into_iter() {
            let span = *error.span();

            let expected = error
                .expected()
                .map(ToString::to_string)
                .collect::<Vec<_>>();

            let Some(found) = error.found() else {
                return Self::UnexpectedEndOfInput;
            };

            errors.push((span, expected, *found));
        }

        Error::ParseError(errors)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UndefinedVariable(name) => write!(f, "variable `{name}` is unbound"),
            Self::EmptyApplication => write!(f, "cannot evaluate empty application `()`"),
            Self::EmptyProcedure => write!(f, "procedure body cannot be empty"),
            Self::ExpectedProcedure(value) => {
                write!(f, "expected a procedure in application but found `{value}`")
            }
            Self::ExpectedValue(expr) => {
                write!(f, "expected a value, but found `{expr}`")
            }
            Self::IncorrectArity(expected, received) => {
                write!(f, "expected {expected} arguments, but found {received}")
            }
            Self::TypeMismatch(expected, received) => {
                write!(
                    f,
                    "expected an value of type '{expected}', but found `{received}`"
                )
            }
            Self::ExpectedList(received) => {
                write!(f, "expected a list, but found `{received}`")
            }
            Self::IndexOutOfBounds(idx) => {
                write!(f, "index {idx} out of bounds")
            }
            Self::UnexpectedEndOfInput => write!(f, "unexpected end of input"),
            Self::ParseError(errors) => {
                for (span, expected, found) in errors {
                    if expected.is_empty() {
                        writeln!(f, "unexpected '{found}' at {span}")?;
                    } else {
                        writeln!(
                            f,
                            "expected one of {expected:?} at {span} but found '{found}'"
                        )?;
                    }
                }

                Ok(())
            }
        }
    }
}
