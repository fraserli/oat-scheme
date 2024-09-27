use std::collections::HashMap;
use std::rc::Rc;

use crate::error::{Error, Result};
use crate::value::Value;

#[derive(Debug, Clone)]
pub struct Environment {
    frames: Vec<HashMap<String, Rc<Value>>>,
}

impl Environment {
    pub fn bind(&mut self, name: &str, value: &Rc<Value>) {
        self.frames
            .last_mut()
            .unwrap()
            .insert(name.to_owned(), Rc::clone(value));
    }

    pub fn get(&self, name: &str) -> Result<Rc<Value>> {
        for scope in self.frames.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Ok(Rc::clone(value));
            }
        }

        Err(Error::UndefinedVariable(name.to_owned()))
    }

    pub fn new_scope(&mut self) {
        self.frames.push(HashMap::new());
    }

    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    pub fn restore(&mut self, depth: usize) {
        self.frames.truncate(depth);
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            frames: vec![HashMap::from_iter(crate::builtin::builtins())],
        }
    }
}
