use std::rc::Rc;

use crate::environment::Environment;
use crate::error::{Error, Result};
use crate::unscheme;
use crate::value::{Procedure, Value};

impl Value {
    pub fn eval(self: &Rc<Self>, env: &mut Environment) -> Result<Rc<Self>> {
        eval(Rc::clone(self), env)
    }

    pub fn eval_to_value(self: &Rc<Self>, env: &mut Environment) -> Result<Rc<Self>> {
        let eval = self.eval(env)?;
        match *eval {
            Self::Void => Err(Error::ExpectedValue(Rc::clone(self))),
            _ => Ok(eval),
        }
    }
}

fn eval(mut value: Rc<Value>, env: &mut Environment) -> Result<Rc<Value>> {
    let initial_stack_depth = env.depth();

    // Loop only repeats during a tail call
    let ret = 'tail_call: loop {
        let (name, args) = match *value {
            Value::Pair((ref car, ref cdr)) => (car, cdr),
            Value::Symbol(ref name) => break env.get(name)?,
            Value::EmptyList => return Err(Error::EmptyApplication),
            _ => break value,
        };

        if let Ok(s) = unscheme!(name => Symbol) {
            match s.as_ref() {
                "define" => break eval_define(args, env)?,
                "and" => break eval_and(args, env)?,
                "or" => break eval_or(args, env)?,
                "lambda" => break eval_lambda(args)?,
                "quote" => break eval_quote(args)?,
                "if" => {
                    let (predicate, (consequent, alternative)) =
                        unscheme!(args => [any, any, any])?;

                    value = if predicate.eval_to_value(env)?.to_bool() {
                        consequent
                    } else {
                        alternative
                    };

                    continue 'tail_call;
                }
                _ => {}
            }
        }

        let procedure = name.eval_to_value(env)?;
        let procedure = match *procedure {
            Value::Procedure(ref p) => p,
            _ => return Err(Error::ExpectedProcedure(Rc::clone(&procedure))),
        };

        match procedure {
            Procedure::Primitive(f) => break f(args, env)?,
            Procedure::Compound { parameters, body } => {
                let args: Vec<Rc<Value>> = args
                    .map(|arg| arg?.eval_to_value(env))
                    .collect::<Result<_>>()?;

                if args.len() != parameters.len() {
                    return Err(Error::IncorrectArity(parameters.len(), args.len()));
                }

                if env.depth() == initial_stack_depth {
                    env.new_scope();
                }

                for (param, arg) in parameters.iter().zip(args) {
                    env.bind(param, &arg);
                }

                debug_assert!(!body.is_empty());

                for expression in &body[..body.len() - 1] {
                    expression.eval(env)?;
                }

                let last = Rc::clone(&body[body.len() - 1]);
                value = last;

                continue 'tail_call;
            }
        }
    };

    env.restore(initial_stack_depth);

    Ok(ret)
}

fn eval_define(args: &Rc<Value>, env: &mut Environment) -> Result<Rc<Value>> {
    if let Ok(((ref name, ref params), ref body)) = unscheme!(args => [Pair, rest]) {
        let name = unscheme!(name => Symbol)?;
        env.bind(&name, &make_lambda(params, body)?);
    } else {
        let (lhs, rhs) = unscheme!(args => [Symbol, any])?;
        let rhs = rhs.eval_to_value(env)?;
        env.bind(&lhs, &rhs);
    }

    Ok(Value::void())
}

fn eval_and(args: &Rc<Value>, env: &mut Environment) -> Result<Rc<Value>> {
    for value in args.as_ref() {
        if !value?.eval_to_value(env)?.to_bool() {
            return Ok(Value::boolean(false));
        }
    }
    Ok(Value::boolean(true))
}

fn eval_or(args: &Rc<Value>, env: &mut Environment) -> Result<Rc<Value>> {
    for value in args.as_ref() {
        if value?.eval_to_value(env)?.to_bool() {
            return Ok(Value::boolean(true));
        }
    }
    Ok(Value::boolean(false))
}

fn eval_lambda(args: &Rc<Value>) -> Result<Rc<Value>> {
    let (parameters, body) = unscheme!(args => Pair)?;
    make_lambda(&parameters, &body)
}

fn eval_quote(args: &Rc<Value>) -> Result<Rc<Value>> {
    unscheme!(args => [any])
}

fn make_lambda(parameters: &Value, body: &Value) -> Result<Rc<Value>> {
    let parameters = parameters
        .map(|p| p.and_then(|p| unscheme!(&p => Symbol)))
        .collect::<Result<_>>()?;

    let body: Vec<Rc<Value>> = body.collect::<Result<_>>()?;

    if body.is_empty() {
        Err(Error::EmptyProcedure)
    } else {
        Ok(Value::procedure(Procedure::Compound { parameters, body }))
    }
}

#[cfg(test)]
mod tests {
    use crate::{parse_one, Environment, Value};

    #[test]
    fn scoping() {
        let mut env = Environment::default();

        parse_one("(define (f x) (define a 0) x)")
            .unwrap()
            .eval(&mut env)
            .unwrap();

        parse_one("(f 0)").unwrap().eval(&mut env).unwrap();

        assert!(env.get("f").is_ok());
        assert!(env.get("x").is_err());
        assert!(env.get("a").is_err());

        parse_one("(define (g f) (eq? f 0))")
            .unwrap()
            .eval(&mut env)
            .unwrap();

        assert_eq!(
            parse_one("(g 0)").unwrap().eval(&mut env).unwrap(),
            Value::boolean(true)
        );
    }
}
