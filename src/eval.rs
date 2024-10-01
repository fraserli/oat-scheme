use gc::Gc;

use crate::environment::Environment;
use crate::error::{Error, Result};
use crate::unscheme;
use crate::value::{PrimitiveProcedure, Procedure, Value};

pub fn eval_to_value(value: Gc<Value>, env: &mut Environment) -> Result<Gc<Value>> {
    let v = eval(value.clone(), env)?;
    match &*v {
        Value::Void => Err(Error::ExpectedValue(value)),
        _ => Ok(v),
    }
}

pub fn eval(mut value: Gc<Value>, env: &mut Environment) -> Result<Gc<Value>> {
    let initial_stack_depth = env.depth();

    // Loop only repeats during a tail call
    let ret: Gc<Value> = 'tail_call: loop {
        let (procedure, args) = match &*value {
            Value::Pair((car, cdr)) => (car, cdr),
            Value::Symbol(name) => break env.get(name)?,
            Value::EmptyList => return Err(Error::EmptyApplication),
            _ => break value.clone(),
        };

        if let Ok(s) = unscheme!(procedure => Symbol) {
            match s.as_ref() {
                "define" => break eval_define(args, env)?,
                "and" => break eval_and(args, env)?,
                "or" => break eval_or(args, env)?,
                "lambda" => break eval_lambda(args, env)?,
                "quote" => break eval_quote(args)?,
                "if" => {
                    let (predicate, (consequent, alternative)) =
                        unscheme!(args => [any, any, any])?;

                    value = if eval_to_value(predicate, env)?.to_bool() {
                        consequent
                    } else {
                        alternative
                    };

                    continue 'tail_call;
                }
                _ => {}
            }
        }

        let procedure = eval_to_value(procedure.clone(), env)?;
        let (parameters, body, captures) = match &*procedure {
            Value::PrimitiveProcedure(PrimitiveProcedure(f)) => break f(args, env)?,
            Value::Procedure(Procedure {
                parameters,
                body,
                captures,
            }) => (parameters, body, captures),
            _ => return Err(Error::ExpectedProcedure(procedure.clone())),
        };

        let args: Vec<Gc<Value>> = args
            .map(|arg| eval_to_value(arg?, env))
            .collect::<Result<_>>()?;

        if args.len() != parameters.len() {
            return Err(Error::IncorrectArity(parameters.len(), args.len()));
        }

        if env.depth() == initial_stack_depth {
            env.new_scope();
        }

        for (param, arg) in parameters.iter().zip(args) {
            env.bind(param, arg);
        }

        for (name, value) in captures {
            env.bind(name, value.clone());
        }

        debug_assert!(!body.is_empty());

        let mut body = body.clone();
        let last = body.remove(body.len() - 1);

        for expr in body {
            eval(expr, env)?;
        }

        value = last;

        continue 'tail_call;
    };

    env.restore(initial_stack_depth);

    Ok(ret)
}

fn eval_define(args: &Gc<Value>, env: &mut Environment) -> Result<Gc<Value>> {
    if let Ok(((ref name, ref params), ref body)) = unscheme!(args => [Pair, rest]) {
        let name = unscheme!(name => Symbol)?;
        let procedure = make_lambda(params, body, env)?;
        env.bind(&name, procedure);
    } else {
        let (lhs, rhs) = unscheme!(args => [Symbol, any])?;
        let rhs = eval_to_value(rhs, env)?;
        env.bind(&lhs, rhs);
    }

    Ok(Value::void())
}

fn eval_and(args: &Gc<Value>, env: &mut Environment) -> Result<Gc<Value>> {
    for value in args.as_ref() {
        if !eval_to_value(value?, env)?.to_bool() {
            return Ok(Value::boolean(false));
        }
    }
    Ok(Value::boolean(true))
}

fn eval_or(args: &Gc<Value>, env: &mut Environment) -> Result<Gc<Value>> {
    for value in args.as_ref() {
        if eval_to_value(value?, env)?.to_bool() {
            return Ok(Value::boolean(true));
        }
    }
    Ok(Value::boolean(false))
}

fn eval_lambda(args: &Gc<Value>, env: &mut Environment) -> Result<Gc<Value>> {
    let (parameters, body) = unscheme!(args => Pair)?;
    make_lambda(&parameters, &body, env)
}

fn eval_quote(args: &Gc<Value>) -> Result<Gc<Value>> {
    unscheme!(args => [any])
}

fn make_lambda(parameters: &Value, body: &Value, env: &mut Environment) -> Result<Gc<Value>> {
    let parameters = parameters
        .map(|p| p.and_then(|p| unscheme!(&p => Symbol)))
        .collect::<Result<Vec<_>>>()?;

    let body: Vec<_> = body.collect::<Result<_>>()?;

    if body.is_empty() {
        return Err(Error::EmptyProcedure);
    }

    let mut captures = Vec::new();
    for expr in &body {
        get_captures(expr, &parameters, env, &mut captures);
    }

    Ok(Value::procedure(parameters, body, captures))
}

fn get_captures(
    value: &Value,
    parameters: &[String],
    env: &mut Environment,
    captures: &mut Vec<(String, Gc<Value>)>,
) {
    let mut stack = vec![value];

    while let Some(v) = stack.pop() {
        match v {
            Value::Symbol(s) => {
                if !parameters.contains(s) {
                    if let Ok(v) = env.get(s) {
                        captures.push((s.to_owned(), v));
                    }
                }
            }
            Value::Pair((car, cdr)) => {
                stack.push(car);
                stack.push(cdr);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_one, Environment, Value};

    #[test]
    fn scoping() -> Result<()> {
        let env = &mut Environment::default();

        eval(parse_one("(define (f x) (define a 0) x)")?, env)?;

        eval(parse_one("(f 0)")?, env)?;

        assert!(env.get("f").is_ok());
        assert!(env.get("x").is_err());
        assert!(env.get("a").is_err());

        eval(parse_one("(define (g f) (eq? f 0))")?, env)?;

        assert_eq!(eval(parse_one("(g 0)")?, env)?, Value::boolean(true));

        Ok(())
    }

    #[test]
    fn closures() -> Result<()> {
        let env = &mut Environment::default();

        eval(
            parse_one(
                "
                (define (make-closure)
                  (define local 0)
                  (define (closure) local)
                  closure)
                ",
            )?,
            env,
        )?;

        assert_eq!(
            eval(parse_one("((make-closure))")?, env)?,
            Value::number(0.0),
        );

        Ok(())
    }
}
