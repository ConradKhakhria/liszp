use crate::{
    read::Value,
    refcount_list
};
use std::{
    collections::HashMap,
    rc::Rc
};
use rug;


type ValueMap = HashMap<String, Rc<Value>>;

pub struct Env {
    globals: ValueMap
}


impl Env {
    pub fn new() -> Self {
        Env {
            globals: HashMap::new()
        }
    }


    /* Getters */

    pub fn get_globals(&self) -> &ValueMap {
        &self.globals
    }


    /* Env-related functions */

    fn resolve(&self, value: &Rc<Value>) -> Rc<Value> {
        /* If 'value' is a name, this substitutes it for the ident's value */

        if let Value::Name(name) = &**value {
            self.globals.get(name).expect(format!("Unbound name '{}'", &name[1..]).as_str()).clone()
        } else {
            value.clone()
        }
    }


    /* Eval */

    pub fn eval(&mut self, expr: &Rc<Value>) -> Rc<Value> {
        /* Evaluates an expression in Env */

        let mut value = expr.clone();

        while let Value::Cons { car: function, cdr: args  } = &*value {
            let args = args.to_list().expect("Liszp: expected a list of arguments");

            println!("{}", &value);

            value = match function.name().as_str() {
                "&car"            => self.car(&args),
                "&cons"           => self.cons(&args),
                "&define"         => self.define_value(&args),
                "&equals?"        => self.values_are_equal(&args),
                "&eval"           => self.eval_quoted(&args),
                "&if"             => self.if_expr(&args),
                "&len"            => self.value_length(&args),
                "no-continuation" => self.no_continuation(&args),
                "&print"          => self.print_value(&args, false),
                "&println"        => self.print_value(&args, true),
                "&quote"          => self.quote_value(&args),
                _                 => self.evaluate_lambda_funcall(function, &args)
            }
        }

        value
    }


    /* Non-built-in function evaluation */

    fn evaluate_lambda_funcall(&self, function: &Rc<Value>, arg_values: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Evaluates the calling of a non-built-in function */

        let function_components = self.resolve(function)
                                                   .to_list()
                                                   .expect("Liszp: function should have syntax (lambda <args> <body>)");

        if function_components.len() != 3 {
            panic!("Liszp: function should have syntax (lambda <args> <body>)");
        } else if function_components[0].name() != "&lambda" {
            panic!("Liszp: attempt to call a non-function value");
        }

        let arg_names = Self::get_arg_names(&function_components[1]);
        let mut arg_map = Self::build_argument_hashmap(&arg_names, arg_values);

        let function_body = &function_components[2];

        self.recursively_bind_args(function_body, &mut arg_map)
    }


    fn get_arg_names(arg_component: &Rc<Value>) -> Vec<String> {
        /* Gets the list of argument names from the argument component */

        match &**arg_component {
            Value::Cons {..} => {
                let values_list = arg_component.to_list().unwrap();
                let mut names = Vec::with_capacity(values_list.len());

                for v in values_list.iter() {
                    match &**v {
                        Value::Name(name) => names.push(name.clone()),
                        _ => panic!("Liszp: Expected name in function argument")
                    }
                }

                names
            }

            Value::Name(name) => {
                vec![ name.clone() ]
            }

            Value::Nil => vec![],

            _ => panic!("Liszp: Function expected a list of arguments or a single argument in ")
        }
    }


    fn build_argument_hashmap(arg_names: &Vec<String>, arg_values: &Vec<Rc<Value>>) -> ValueMap {
        /* Builds a map from argument names to argument values */

        let mut hashmap = HashMap::new();

        if arg_names.len() != arg_values.len() {
            panic!("Function takes {} arguments but received {}", arg_names.len(), arg_values.len());
        }

        for i in 0..arg_names.len() {
            hashmap.insert(arg_names[i].clone(), arg_values[i].clone());
        }

        hashmap
    }


    fn recursively_bind_args(&self, expr: &Rc<Value>, arg_map: &mut ValueMap) -> Rc<Value> {
        /* Returns function_body but with argument names replaced with their values */

        match &**expr {
            Value::Name(name) => {
                if let Some(value) = arg_map.get(name) {
                    value.clone()
                } else {
                    expr.clone()
                }
            },

            Value::Cons { car, cdr } => {
                if car.name() == "&lambda" {
                    let lambda_components = expr.to_list().expect("Liszp: malformed lambda expression");
                    let arg_component = &lambda_components[1];
                    let body_component = &lambda_components[2];

                    let shadowed_arguments = Self::remove_shadowed_arguments(arg_component, arg_map);

                    let body_with_bound_arguments = self.recursively_bind_args(body_component, arg_map);

                    arg_map.extend(shadowed_arguments);

                    body_with_bound_arguments
                } else {
                    Rc::new(Value::Cons {
                        car: self.recursively_bind_args(car, arg_map),
                        cdr: self.recursively_bind_args(cdr, arg_map)
                    })
                }
            }

            _ => expr.clone()
        }
    }


    fn remove_shadowed_arguments(arg_component: &Rc<Value>, arg_map: &mut ValueMap) -> ValueMap {
        /* Removes any arguments from arg_map that are shadowed in lambda_components */

        let mut shadowed_args = HashMap::new();

        for arg_name in Self::get_arg_names(arg_component) {
            if let Some(removed_value) = arg_map.remove(&arg_name) {
                shadowed_args.insert(arg_name, removed_value);
            }
        }

        shadowed_args
    }


    /* built-in functions */

    fn car(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Gets the car of a cons pair */

        match args.as_slice() {
            [continuation, xs] => {
                let resolved = self.resolve(xs);

                let xs = match &*resolved {
                    Value::Quote(cons) => cons.clone(),
                    _ => resolved
                };

                let car = match &*xs {
                    Value::Cons { car, .. } => car,
                    _ => panic!("Liszp: function 'cons' expected to receive cons pair")
                };

                refcount_list![ continuation, car ]
            }

            _ => panic!("Liszp: function 'car' takes 1 argument")
        }
    }


    fn cons(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Creates a cons pair */

        match args.as_slice() {
            [continuation, car, cdr] => {
                let car = self.resolve(car);
                let cdr = self.resolve(cdr);

                let cons_pair = Value::Quote(
                    Rc::new(Value::Cons {
                        car,
                        cdr: if let Value::Quote(v) = &*cdr {
                            v.clone()
                        } else {
                            cdr
                        }
                    })
                );

                refcount_list![ continuation.clone(), cons_pair.rc() ]
            }

            _ => panic!("Liszp: function 'cons' expected 2 arguments")
        }
    }


    fn define_value(&mut self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Defines a value in self.globals */

        if args.len() != 3 {
            panic!("Liszp: expected syntax (def <name> <value>)");
        }

        let continuation = &args[0];
        let name = &args[1];
        let value = &args[2];

        if let Value::Name(name) = &**name {
            self.globals.insert(name.clone(), value.clone());
        } else {
            panic!("Liszp: expected name in def expression");
        }

        refcount_list![ continuation.clone(), Value::Nil.rc() ]
    }


    fn eval_quoted(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Evaluates a quoted value */

        match args.as_slice() {
            [continuation, quoted_value] => {
                let value = if let Value::Quote(v) = &*self.resolve(quoted_value) {
                    v.clone()
                } else {
                    quoted_value.clone()
                };

                refcount_list![ continuation, &value ]
            }

            _ => panic!("Liszp: function 'quote' takes exactly one argument")
        }
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
            self.resolve(&args[0])
        } else {
            unreachable!()
        }
    }


    fn print_value(&self, args: &Vec<Rc<Value>>, newline: bool) -> Rc<Value> {
        /* Prints a value, optionally with a newline */

        if args.len() != 2 {
            panic!("Function print{} takes 1 argument only", if newline { "ln" } else { "" });
        }

        let continuation = &args[0];
        let value = self.resolve(&args[1]);

        if newline {
            println!("{}", value);
        } else {
            print!("{}", value);
        }

        refcount_list![ continuation.clone(), value]
    }


    fn quote_value(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Quotes a value */

        match args.as_slice() {
            [continuation, value] => {
                let quoted_value = Value::Quote(self.resolve(value)).rc();

                refcount_list![ continuation, &quoted_value ]
            }

            _ => panic!("Liszp: function 'quote' takes exactly one value")
        }
    }


    fn values_are_equal(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Compares two values */

        match args.as_slice() {
            [continuation, x, y] => {
                let result = Value::Bool(self.resolve(x) == self.resolve(y)).rc();

                refcount_list![ continuation, &result ]
            },

            _ => panic!("Liszp: Function 'equals?' takes exactly 2 parameters")
        }
    }


    fn value_length(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Gets the length of a value, if applicable */

        match args.as_slice() {
            [continuation, xs] => {
                let length = rug::Integer::from(self.resolve(xs).len());
                let value = Value::Integer(length).rc();

                refcount_list![ continuation, &value ]
            },

            _ => panic!("Liszp: function 'len' takes exactly one value")
        }
    }
}
