use crate::parse::Value;
use crate::eval::{ builtin, operators::{ arithmetic, boolean, comparison } };

use std::collections::{HashMap, LinkedList};
use std::rc::Rc;

#[macro_export]
macro_rules! remove_amp {
    ($string:expr) => {
        {
            let temp_len = $string.len();
            &($string)[..temp_len-1]
        }
    };
}

pub (in crate::eval) type Env = HashMap<String, Rc<Value>>;

/* Generic helper functions */

pub (in crate::eval) fn resolve_value(value: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* If value is a Value::Name, it is reduced to the non-name value */

    let mut shared = Rc::clone(value);

    while let Value::Name(name) = &*shared {
        shared = env.get(name)
                    .expect(&format!("Unbound value name '{}'", remove_amp!(name))[..])
                    .clone();
    }

    return shared;
}

fn bind_variables(function: Rc<Value>, args: &Rc<Value>) -> Rc<Value> {
   /* Binds the variables in 'args' to a function
    *
    * arguments
    * ---------
    * - function: the lambda expression which has been called
    * - args: the arguments supplied in calling 'function'
    *
    * returns
    * -------
    * The body of 'function', with each argument name replaced with
    * its Rc<Value> from 'args'.
    */

    fn rec_bind_var(expr: &Rc<Value>, name: String, value: Rc<Value>) -> Rc<Value> {
        /* Recursively replaces instances of Rc<Value::Name(name)> with value */

        match &**expr {
            Value::Name(string) => {
                return if *string == name {
                    value
                } else {
                    Rc::clone(expr)
                };
            },

            Value::Cons { car, cdr } => {
                if &(**car).name()[..] == "lambda&" {
                    // The only reason a Value::Cons(name) wouldn't be bound to 'value'
                    // is if the name is shadowed in a lambda expression. To check this,
                    // we see if this lambda expression contains an arg whose name is 'name'

                    let args = if let Value::Cons { car: asv, .. } = &**cdr {
                        if let Value::Name(_) = &**asv {
                            let mut temp_list = LinkedList::new();
                            temp_list.push_back(Rc::clone(&asv));
                            temp_list
                        } else {
                            asv.to_list().expect("Expected lambda function to have args")
                        }
                    } else {
                        panic!("Expected lambda function to have args");
                    };

                    for arg in args.iter() {
                        if let Value::Name(n) = &**arg {
                            if &n[..] == &name[..] {
                                return Rc::clone(expr);
                            }
                        }
                    }
                }

                return Rc::new(Value::Cons {
                    car: rec_bind_var(&car, name.clone(), Rc::clone(&value)),
                    cdr: rec_bind_var(&cdr, name, Rc::clone(&value))
                });
            },

            _ => return expr.clone()
        };
    }

    let function_list = function.to_list().expect("Expected lambda expression");

    if function_list.len() != 3 {
        panic!("Liszp: lambda expression expected 2 arguments (lambda <args> <body>), received {}", function_list.len());
    }

    let mut flist_iter = function_list.iter();

    flist_iter.next(); // Lambda keyword
    let function_args_val = flist_iter.next().unwrap();
    let function_body_val = flist_iter.next().unwrap();

    let supplied_args = args.to_list().expect("Expected function to be called with args");
    let function_args = if let Value::Name(_) = &**function_args_val {
        let mut list = LinkedList::new();
        list.push_back(Rc::clone(function_args_val));

        list
    } else {
        function_args_val.to_list()
                         .expect(&format!("Function not defined with arguments (received expr {})", function_args_val)[..])
    };

    if function_args.len() != supplied_args.len() {
        panic!("Function takes {} arguments but received {}", function_args.len(), supplied_args.len());
    }

    // Apply the arguments
    let mut bound_variables_body = (**function_body_val).clone().refcounted();

    for (name, val) in function_args.iter().zip(supplied_args.iter()) {
        if let Value::Name(n) = &**name {
            bound_variables_body = rec_bind_var(&bound_variables_body, n.clone(), Rc::clone(val));
        } else {
            panic!("Expected defined function argument to be variable name");
        }
    }

    return bound_variables_body;
}

fn no_continuation(parameters: Rc<Value>, env: &mut HashMap<String, Rc<Value>>) -> Rc<Value> {
    /* Ends an expression's evaluation */

    if let Value::Cons { car, cdr } = &*parameters {
        if let Value::Nil = **cdr {
            return resolve_value(car, env);
        }
    }

    panic!("Function no-continuation should be supplied with exactly one argument")
}


pub fn eval(supplied: Rc<Value>, env: &mut Env) -> Rc<Value> {
   /* Evaluates an expression
    *
    * args
    * ----
    * - supplied: the expression to evaluate
    *
    * returns
    * -------
    * The evaluated expression (i.e. the supplied function is
    * reduced to an atomic expr)
    */

    let mut value = Rc::clone(&supplied);

    macro_rules! evaluate {
        ($value_to_add:expr) => { {
            value = $value_to_add;
            continue;
        } };
    }

    while let Value::Cons { car: function_value, cdr: args } = &*value {
        match &function_value.name()[..] {
            "def&"                     => evaluate!(builtin::define_value(args, env)),
            "print&"|"println&"        => evaluate!(builtin::print_value(args, env, function_value.name())),
            "if&"                      => evaluate!(builtin::if_expr(args, env)),
            "cons&"                    => evaluate!(builtin::cons(args, env)),
            "no-continuation"          => evaluate!(no_continuation(Rc::clone(args), env)),
            "+&"|"-&"|"*&"|"/&"|"%&"   => evaluate!(arithmetic(function_value.name(), Rc::clone(args), env)),
            "not&"|"and&"|"or&"|"xor&" => evaluate!(boolean(function_value.name(), Rc::clone(args), env)),
            "<&"|">&"|"<=&"|">=&"|"==&"|"!=&" => evaluate!(comparison(function_value.name(), Rc::clone(args), env)),
            _ => {}
        }

        let function = resolve_value(function_value, env);
        value = bind_variables(function, args);
    }

    return value;
}
