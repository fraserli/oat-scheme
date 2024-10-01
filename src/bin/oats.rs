use std::fs::File;
use std::io::{Read, Write};

use oat_scheme::{eval, parse, Environment, Value};

fn run(input: &str, env: &mut Environment) {
    let values = match parse(input) {
        Ok(v) => v,
        Err(err) => {
            println!("error: {err}");
            return;
        }
    };

    for value in values {
        match eval(value, env) {
            Ok(value) => {
                if *value != Value::Void {
                    println!("{}", value)
                }
            }
            Err(err) => {
                println!("error: {}", err);
            }
        }
    }
}

fn repl() {
    let mut env = Environment::default();

    loop {
        print!("> ");
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        run(&input, &mut env)
    }
}

fn main() {
    if let Some(path) = std::env::args().nth(1) {
        let mut file = File::open(path).unwrap();

        let mut input = String::new();
        file.read_to_string(&mut input).unwrap();

        run(&input, &mut Environment::default());
    } else {
        repl();
    }
}
