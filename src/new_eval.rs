use crate::parse::Value;

use std::collections::{HashMap, LinkedList};
use std::rc::Rc;

macro_rules! remove_amp {
    ($string:expr) => {
        {
            let temp_len = $string.len();
            &($string)[..temp_len-1]
        }
    };
}

/* Generic helper functions */

fn resolve_value(value: &Rc<Value>, env: &HashMap<String, Rc<Value>>) -> Rc<Value> {
    /* If value is a Value::Name, it is reduced to the non-name value */

    let mut shared = Rc::clone(value);

    while let Value::Name(name) = &*shared {
        shared = env.get(name)
                    .expect(&format!("Unbound value name {}", remove_amp!(name))[..])
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

        return match &**expr {
            Value::Name(string) => {
                if *string == name {
                    value
                } else {
                    Rc::clone(expr)
                }
            },

            Value::Cons { car, cdr } => {
                Rc::new(Value::Cons {
                    car: rec_bind_var(&car, name.clone(), Rc::clone(&value)),
                    cdr: rec_bind_var(&cdr, name, Rc::clone(&value))
                })
            },

            _ => expr.clone()
        };
    }

    let function_list = function.to_list();

    if function_list.len() != 3 {
        panic!("Liszp: lambda expression expected 2 arguments (lambda <args> <body>), received {}", function_list.len());
    }

    let mut flist_iter = function_list.iter();

    flist_iter.next(); // Lambda keyword
    let function_args_val = flist_iter.next().unwrap();
    let function_body_val = flist_iter.next().unwrap();


    let supplied_args = if args.is_cons() {
        args.to_list()
    } else if args.is_nil() {
        LinkedList::new()
    } else {
        panic!("Expected function to be called with arguments");
    };

    let function_args = if function_args_val.is_cons() {
        function_args_val.to_list()
    } else if function_args_val.is_nil() {
        LinkedList::new()
    } else if let Value::Name(_) = &**function_args_val {
        let mut list = LinkedList::new();
        list.push_front(Rc::clone(function_args_val));

        list
    } else {
        panic!("Function not defined with arguments (received {})", function_args_val);
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

/* Built-in functions */

fn define_value(parameters: &Rc<Value>, env: &mut HashMap<String, Rc<Value>>) -> Rc<Value> {
    /* Adds a value to the global namespace */

    let parameters_list = if !parameters.is_cons() {
        panic!("Liszp: Expected def expression with syntax (def <name> <value>)");
    } else if parameters.len() != 2 {
        panic!("Liszp: def expression received {} arguments but expected 2", parameters.len());
    } else {
        parameters.to_list()
    };

    let mut p_iter = parameters_list.iter();

    let name_value = p_iter.next().unwrap();
    let body_value = p_iter.next().unwrap();

    let name = if let Value::Name(n) = &**name_value {
        n
    } else {
        panic!("Liszp: Expected name in def expr");
    };

    env.insert(name.clone(), Rc::clone(body_value));

    return Value::Nil.refcounted();
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


fn arithmetic(op: String, parameters: Rc<Value>, env: &mut HashMap<String, Rc<Value>>) -> Rc<Value> {
    /* Evaluates an arithmetic expression */

    let mut numbers = LinkedList::new();
    let mut floats = false;

    for param in parameters.to_list().iter() {
        match &**param {
            Value::Float(_) => {
                floats = true;
                numbers.push_front(Rc::clone(param));
            },

            Value::Integer(_) => {
                numbers.push_front(Rc::clone(param))
            },

            Value::Name(_) => {
                numbers.push_front(Rc::clone(&resolve_value(param, env)))
            },

            _ => panic!("Expected number literal or variable containing number in '{}' expression", op)
        }
    }

    if floats {
        
    }




    parameters
}

pub fn eval(supplied: Rc<Value>, env: &mut HashMap<String, Rc<Value>>) -> Rc<Value> {
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

    while let Value::Cons { car: function_value, cdr: args } = &*value {
        macro_rules! evaluate {
            ($value_to_add:expr) => { {
                value = $value_to_add;
                continue;
            } };
        }

        match &function_value.name()[..] {
            "def&"                   => evaluate!(define_value(args, env)),
            "no-continuation"        => evaluate!(no_continuation(Rc::clone(args), env)),
            "+&"|"-&"|"*&"|"/&"|"%&" => evaluate!(arithmetic(function_value.name(), Rc::clone(args), env)),
            _ => {}
        }

        let function = resolve_value(function_value, env);
        value = bind_variables(function, args);
    }

    return value;
}
