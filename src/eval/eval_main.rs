use crate::read::Value;
use crate::eval::{ builtin, operators::{ arithmetic, boolean, comparison } };

use std::collections::{ HashMap, LinkedList };
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

#[macro_export]
macro_rules! unroll_parameters {
    { $params:expr, $msg:expr, $cont:literal ; $( $x:ident ),+ } => {
        let parameter_list  = $params.to_list().expect($msg);
        let mut plist_iter  = parameter_list.iter();
        let mut ident_count = 0;

        $(
            let $x;
            ident_count += 1;
        )*

        if ident_count != parameter_list.len() {
            panic!("{}:\nrecieved {} args", $msg, parameter_list.len() - if $cont { 1 } else { 0 });
        }

        $(
            $x = plist_iter.next().unwrap();
        )*
    };
}

pub (in crate::eval) type Env = HashMap<String, Rc<Value>>;

/* Generic helper functions */

pub (in crate::eval) fn resolve_value(value: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* If value is a Value::Name, it is reduced to the non-name value */

    let mut shared = Rc::clone(value);

    while let Value::Name(name) = &*shared {
        shared = Rc::clone(env.get(name)
                                   .expect(&format!("Unbound value name '{}'", remove_amp!(name))[..]));
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

    fn rec_bind_var(expr: &Rc<Value>, name: &String, value: Rc<Value>) -> Rc<Value> {
        /* Recursively replaces instances of Rc<Value::Name(name)> with value */

        match &**expr {
            Value::Name(string) => {
                return if *string == *name {
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
                            asv.to_list().expect("Liszp: expected lambda function to have args")
                        }
                    } else {
                        panic!("Liszp: expected lambda function to have args");
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
                    car: rec_bind_var(&car, &name, Rc::clone(&value)),
                    cdr: rec_bind_var(&cdr, &name, Rc::clone(&value))
                });
            },

            _ => return Rc::clone(expr)
        };
    }

    let function_list = function.to_list().expect("Liszp: expected lambda expression");

    if function_list.len() != 3 {
        panic!("Liszp: lambda expression expected 2 arguments (lambda <args> <body>), received {}", function_list.len());
    }

    let mut flist_iter = function_list.iter();

    flist_iter.next(); // Lambda keyword
    let function_args_val = flist_iter.next().unwrap();
    let function_body_val = flist_iter.next().unwrap();

    let supplied_args = args.to_list().expect("Liszp: expected function to be called with args");
    let function_args = if let Value::Name(_) = &**function_args_val {
        let mut list = LinkedList::new();
        list.push_back(Rc::clone(function_args_val));

        list
    } else {
        function_args_val.to_list()
                         .expect(&format!("Liszp: function not defined with arguments (received expr {})", function_args_val)[..])
    };

    if function_args.len() != supplied_args.len() {
        panic!("Liszp: function takes {} arguments but received {}", function_args.len(), supplied_args.len());
    }

    // Apply the arguments
    let mut bound_variables_body = Rc::clone(function_body_val);

    for (name, val) in function_args.iter().zip(supplied_args.iter()) {
        if let Value::Name(n) = &**name {
            bound_variables_body = rec_bind_var(&bound_variables_body, n, Rc::clone(val));
        } else {
            panic!("Liszp: expected argument in function literal to be a variable name");
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

    panic!("Function no-continuation should be supplied with exactly one argument");
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

    while let Value::Cons { car: function_value, cdr: args } = &*value {
        value = match &function_value.name()[..] {
            "def&"                            => builtin::define_value(args, env),
            "print&"|"println&"               => builtin::print_value(args, env, function_value.name()),
            "if&"                             => builtin::if_expr(args, env),
            "equals?&"                        => builtin::compare_values(args, env),
            "len&"                            => builtin::get_length(args, env),
            "quote&"                          => builtin::quote(args, env),
            "eval&"                           => builtin::eval_quoted(args, env),
            "cons&"                           => builtin::cons(args, env),
            "car&"|"first&"                   => builtin::car(args, env, function_value.name()),
            "cdr&"|"rest&"                    => builtin::cdr(args, env, function_value.name()),
            "null?&"|"empty?&"|"nil?&"        => builtin::is_nil(args, env),
            "cons?&"|"pair?&"                 => builtin::is_cons(args, env),
            "int?&"                           => builtin::is_int(args, env),
            "float?&"                         => builtin::is_float(args, env),
            "str?&"                           => builtin::is_string(args, env),
            "bool?&"                          => builtin::is_bool(args, env),
            "quote?&"                         => builtin::is_quote(args, env),
            "name?&"                          => builtin::is_name(args, env),
            "no-continuation"                 => no_continuation(Rc::clone(args), env),
            "+&"|"-&"|"*&"|"/&"|"%&"          => arithmetic(function_value.name(), Rc::clone(args), env),
            "not&"|"and&"|"or&"|"xor&"        => boolean(function_value.name(), Rc::clone(args), env),
            "<&"|">&"|"<=&"|">=&"|"==&"|"!=&" => comparison(function_value.name(), Rc::clone(args), env),
            _                                 => bind_variables(resolve_value(function_value, env), args)
        };
    }

    return value;
}
