use std::collections::HashMap;
use std::rc::Rc;

use crate::error::{Error, Result};
use crate::value::Value;

#[derive(Debug, Clone)]
pub struct Environment {
    env: HashMap<String, Rc<Value>>,
}

impl Environment {
    pub fn bind(&mut self, name: &str, value: &Rc<Value>) {
        self.env.insert(name.to_owned(), Rc::clone(value));
    }

    pub fn get(&self, name: &str) -> Result<Rc<Value>> {
        Ok(Rc::clone(
            self.env
                .get(name)
                .ok_or(Error::UndefinedVariable(name.to_owned()))?,
        ))
    }

    pub fn new_scope(&self) -> Self {
        self.clone()
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            env: HashMap::from_iter(crate::builtin::builtins()),
        }
    }
}
