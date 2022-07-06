use crate::{
    read::Value,
    refcount_list
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


    /* Getters */

    pub fn get_globals(&self) -> &HashMap<String, Rc<Value>> {
        &self.globals
    }


    /* Env-related functions */

    fn resolve(&self, value: &Rc<Value>) -> Rc<Value> {
        /* If 'value' is a name, this substitutes it for the ident's value */

        Rc::clone(
        if let Value::Name(name) = &**value {
                self.globals.get(name).expect(format!("Unbound name '{}'", &name[1..]).as_str())
            } else {
                value
            }
        )
    }


    /* Eval */

    pub fn eval(&mut self, expr: &Rc<Value>) -> Rc<Value> {
        /* Evaluates an expression in Env */

        let mut value = Rc::clone(expr);

        while let Value::Cons { car: function, cdr: args  } = &*value {
            let args = args.to_list().expect("Liszp: expected a list of arguments");

            value = match function.name().as_str() {
                "&define"         => self.define_value(&args),
                "&if"             => self.if_expr(&args),
                "no-continuation" => self.no_continuation(&args),
                "&print"          => self.print_value(&args, false),
                "&println"        => self.print_value(&args, true),
                name              => {
                    println!("{}", name);
                    todo!()
                }
            }
        }

        value
    }


    /* built-in functions */

    fn define_value(&mut self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Defines a value in self.globals */

        if args.len() != 3 {
            panic!("Liszp: expected syntax (def <name> <value>)");
        }

        let continuation = &args[0];
        let name = &args[1];
        let value = &args[2];

        if let Value::Name(name) = &**name {
            self.globals.insert(name.clone(), Rc::clone(value));
        } else {
            panic!("Liszp: expected name in def expression");
        }

        refcount_list![ Rc::clone(continuation), Value::Nil.rc() ]
    }


    fn if_expr(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Evaluates an if expression */
    
        if args.len() != 3 {
            panic!("Liszp: if expression has syntax (if <condition> <true case> <false case>)");
        }

        let cond = self.resolve(&args[0]);
        let true_case = self.resolve(&args[1]);
        let false_case = self.resolve(&args[2]);

        if let Value::Bool(b) = &*cond {
            if *b {
                true_case
            } else {
                false_case
            }
        } else {
            panic!("if expression expected a boolean condition")
        }
    }


    fn no_continuation(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* The final stage of a trampolined evaluation */

        if args.len() == 1 {
            Rc::clone(&args[0])
        } else {
            unreachable!()
        }
    }


    fn print_value(&self, args: &Vec<Rc<Value>>, newline: bool) -> Rc<Value> {
        /* Prints a value, optionally with a newline */

        if args.len() != 2 {
            panic!("Function print{} takes 1 argument only", if newline { "ln" } else { "" });
        }

        let continuation = self.resolve(&args[0]);
        let value = self.resolve(&args[1]);

        if newline {
            println!("{}", value);
        } else {
            print!("{}", value);
        }

        refcount_list![continuation, value]
    }
}
