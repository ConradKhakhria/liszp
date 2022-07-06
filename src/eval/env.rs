use crate::{
    read::Value
};
use std::{
    collections::HashMap,
    rc::Rc
};

pub struct Env {
    globals: HashMap<String, Rc<Value>>
}


impl Env {
    pub fn new() -> Self {
        Env {
            globals: HashMap::new()
        }
    }


    pub fn eval(&mut self, expr: &Rc<Value>) -> Rc<Value> {
        /* Evaluates an expression in Env */

        let mut value = Rc::clone(expr);

        while let Value::Cons { car: function, cdr: args  } = &*value {
            let args = args.to_list().expect("Liszp: expected a list of arguments");

            value = match function.name().as_str() {
                "&define" => self.define_value(&args),

                _ => todo!()
            }
        }

        value
    }


    /* Env-related built-in functions */

    fn define_value(&mut self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Defines a value in self.globals */

        if args.len() != 3 {
            panic!("Liszp: expected syntax (def <name> <value>)");
        }

        if let Value::Name(name) = &*args[1] {
            self.globals.insert(name.clone(), Rc::clone(&args[2]));
        } else {
            panic!("Liszp: expected name in def expression");
        }

        crate::refcount_list![ Rc::clone(&args[0]), Value::Nil.rc() ]
    }
}
