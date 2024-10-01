use chumsky::prelude::*;
use chumsky::text::int;
use gc::Gc;
use text::whitespace;

use crate::error::{Error, Result};
use crate::value::Value;

pub fn parse_one(input: &str) -> Result<Gc<Value>> {
    parser()
        .parse(input.trim())
        .into_result()
        .map_err(Error::from_parse_errors)
}

pub fn parse(input: &str) -> Result<Vec<Gc<Value>>> {
    let program_parser = parser()
        .separated_by(whitespace())
        .collect()
        .then_ignore(end());

    program_parser
        .parse(input.trim())
        .into_result()
        .map_err(Error::from_parse_errors)
}

fn parser<'a>() -> impl Parser<'a, &'a str, Gc<Value>, extra::Err<Rich<'a, char>>> {
    recursive(|expression| {
        let boolean = choice((
            just("#t").then(just("rue").or_not()).to(true),
            just("#f").then(just("alse").or_not()).to(false),
        ))
        .map(Value::Boolean)
        .labelled("boolean");

        let character = just("#\\")
            .ignore_then(choice((
                just("newline").to('\n'),
                just("space").to(' '),
                any(),
            )))
            .map(Value::Character)
            .labelled("character");

        let number = just('-')
            .or_not()
            .then(choice((
                int(10).then(just('.')).then(int(10)).to(()),
                int(10).then(just('.')).to(()),
                int(10).to(()),
                just('.').then(int(10)).to(()),
            )))
            .to_slice()
            .map(str::parse)
            .unwrapped()
            .map(Value::Number)
            .labelled("number");

        let escape = just('\\').ignore_then(choice((
            just('n').to('\n'),
            just('t').to('\t'),
            just('"'),
            just('\\'),
        )));

        let string = choice((none_of("\\\""), escape))
            .repeated()
            .collect()
            .padded_by(just('"'))
            .map(Value::String)
            .labelled("string");

        let symbol = choice((
            none_of("|\\")
                .repeated()
                .at_least(1)
                .to_slice()
                .padded_by(just('|')),
            none_of(" \t\r\n|()\";'#").repeated().at_least(1).to_slice(),
        ))
        .map(|s: &str| Value::Symbol(s.to_owned()))
        .labelled("symbol");

        let quote = just('\'')
            .ignore_then(expression.clone())
            .map(|expr| {
                Value::Pair((
                    Value::symbol("quote"),
                    Value::pair(&expr, &Value::empty_list()),
                ))
            })
            .labelled("quote");

        let atom = choice((boolean, character, number, string, symbol, quote));

        let list = expression
            .padded()
            .repeated()
            .collect::<Vec<_>>()
            .map(|v| {
                v.into_iter().rev().fold(Value::EmptyList, |acc, expr| {
                    Value::Pair((expr, Gc::new(acc)))
                })
            })
            .delimited_by(just('('), just(')'))
            .labelled("list");

        choice((atom, list)).map(Gc::new)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheme;

    macro_rules! assert_parse {
        ($input:literal, $value:expr) => {
            assert_eq!(*parse_one($input).unwrap(), $value);
        };
    }

    #[test]
    fn parse_boolean() {
        assert_parse!("#true", Value::Boolean(true));
        assert_parse!("#t", Value::Boolean(true));
        assert_parse!("#false", Value::Boolean(false));
        assert_parse!("#f", Value::Boolean(false));
    }

    #[test]
    fn parse_number() {
        assert_parse!("123456", Value::Number(123456.0));
        assert_parse!("123.456", Value::Number(123.456));
        assert_parse!("123.", Value::Number(123.0));
        assert_parse!("123.0", Value::Number(123.0));
        assert_parse!("0.456", Value::Number(0.456));
        assert_parse!(".456", Value::Number(0.456));
    }

    #[test]
    fn parse_string() {
        assert_parse!(r#""""#, Value::String("".into()));
        assert_parse!(r#""\t\n\\\"""#, Value::String("\t\n\\\"".into()));
        assert_parse!(r#""a b c d""#, Value::String("a b c d".into()));
    }

    #[test]
    fn parse_character() {
        assert_parse!("#\\a", Value::Character('a'));
        assert_parse!("#\\space", Value::Character(' '));
        assert_parse!("#\\newline", Value::Character('\n'));
    }

    #[test]
    fn parse_symbol() {
        assert_parse!("a", Value::Symbol("a".into()));
        assert_parse!("|a b|", Value::Symbol("a b".into()));
        assert_parse!(
            "!$%&*+-./:<=>?@^_~",
            Value::Symbol("!$%&*+-./:<=>?@^_~".into())
        );
    }

    #[test]
    fn parse_list() {
        assert_parse!("()", Value::EmptyList);
        assert_parse!("(a)", *scheme!(a));
        assert_parse!("((a))", *scheme!((a)));
        assert_parse!("(a 2 \"c\" (1 2 3))", *scheme!(a 2 "c" (1 2 3)));
        assert_parse!("((a) b)", *scheme!((a) b));
    }

    #[test]
    fn parse_quote() {
        assert_parse!("'a", *scheme!(quote a));
        assert_parse!("''a", *scheme!(quote (quote a)));
        assert_parse!("'(1 2 3)", *scheme!(quote (1 2 3)));
        assert_parse!("'(''1 2)", *scheme!(quote ((quote (quote 1)) 2)));
    }
}
