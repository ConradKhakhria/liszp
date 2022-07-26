use crate::error::Error;
use crate::eval::Evaluator;
use crate::new_error;
use crate::refcount_list;
use crate::value::Value;
use std::rc::Rc;


pub fn car(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Gets the car of a cons pair */

    let args = evaluator.resolve_globals(args);

    match args.as_slice() {
        [continuation, xs] => {
            match &**xs {
                Value::Cons { car, .. } => Ok(refcount_list![ continuation, car ]),
                _ => new_error!("Liszp: function 'cons' expected to receive cons pair").into()
            }
        },

        _ => new_error!("Liszp: function 'car' takes 1 argument").into()
    }
}


pub fn cdr(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Gets the cdr of a cons pair */

    let args = evaluator.resolve_globals(args);

    match args.as_slice() {
        [continuation, xs] => {
            match &**xs {
                Value::Cons { cdr, .. } => Ok(refcount_list![ continuation, cdr ]),
                _ => new_error!("Liszp: function 'cons' expected to receive cons pair").into()
            }
        },

        _ => new_error!("Liszp: function 'cdr' takes 1 argument").into()
    }
}


pub fn cons(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Creates a cons pair */

    let args = evaluator.resolve_globals(args);

    match args.as_slice() {
        [continuation, car, cdr] => {
            Ok(refcount_list![ continuation.clone(), Value::cons(car, cdr).rc() ])
        }

        _ => new_error!("Liszp: function 'cons' expected 2 arguments").into()
    }
}


pub fn eval_quoted(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Evaluates a quoted value */

    let args = evaluator.resolve_globals(args);

    match args.as_slice() {
        [continuation, quoted_value] => {
            let value = todo!();

            Ok(refcount_list![ continuation, &value ])
        }

        _ => new_error!("Liszp: function 'quote' takes exactly one argument").into()
    }
}


pub fn if_expr(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Evaluates an if expression */

    let args = evaluator.resolve_globals(args);

    match args.as_slice() {
        [cond, true_case, false_case] => {
            if let Value::Bool(b) = &**cond {
                if *b {
                    Ok(true_case.clone())
                } else {
                    Ok(false_case.clone())
                }
            } else {
                new_error!("if expression expected a boolean condition").into()
            }
        },

        _ => new_error!("Liszp: if expression has syntax (if <condition> <true case> <false case>)").into()
    }
}


pub fn panic(args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
    /* Panics */

    match args.as_slice() {
        [_, msg] => panic!("{}", msg),
        _ => new_error!("Liszp: expected syntax (panic <message>)").into()
    }
}


pub fn print_value(args: &Vec<Rc<Value>>, evaluator: &Evaluator, newline: bool) -> Result<Rc<Value>, Error> {
    /* Prints a value, optionally with a newline */

    let args = evaluator.resolve_globals(args);

    match args.as_slice() {
        [continuation, value] => {
            if newline {
                println!("{}", value);
            } else {
                print!("{}", value);
            }
        
            Ok(refcount_list![ continuation.clone(), value.clone()])
        },

        _ => new_error!("Function print{} takes 1 argument only", if newline { "ln" } else { "" }).into()
    }
}


pub fn quote_value(args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
    /* Quotes a value */

    match args.as_slice() {
        [continuation, value] => Ok(refcount_list![ continuation, value ]),

        _ => new_error!("Liszp: function 'quote' takes exactly one value").into()
    }
}


pub fn values_are_equal(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Compares two values */

    let args = evaluator.resolve_globals(args);

    match args.as_slice() {
        [continuation, x, y] => {
            Ok(refcount_list![ continuation.clone(), Value::Bool(x == y).rc() ])
        },

        _ => new_error!("Liszp: Function 'equals?' takes exactly 2 parameters").into()
    }
}


pub fn value_is_bool(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Returns whether a value is a bool */

    let args = evaluator.resolve_globals(args);

    match args.as_slice() {
        [continuation, value] => {
            let result = match &**value {
                Value::Bool(_) => true,
                _ => false
            };

            Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
        },

        _ => new_error!("Liszp: function 'bool?' takes exactly one argument").into()
    }
}


pub fn value_is_cons(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Returns whether a value is a cons pair */

    let args = evaluator.resolve_globals(args);

    match args.as_slice() {
        [continuation, value] => {
            let result = match &**value {
                Value::Cons {..} => true,
                _ => false
            };

            Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
        },

        _ => new_error!("Liszp: function 'cons?' takes exactly one argument").into()
    }
}


pub fn value_is_float(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {   
    /* Returns whether a value is a float */

    let args = evaluator.resolve_globals(args);

    match args.as_slice() {
        [continuation, value] => {
            let result = match &**value {
                Value::Float(_) => true,
                _ => false
            };

            Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
        },

        _ => new_error!("Liszp: function 'float?' takes exactly one argument").into()
    }
}


pub fn value_is_int(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Returns whether a value is an int */

    let args = evaluator.resolve_globals(args);

    match args.as_slice() {
        [continuation, value] => {
            let result = match &**value {
                Value::Integer(_) => true,
                _ => false
            };

            Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
        },

        _ => new_error!("Liszp: function 'int?' takes exactly one argument").into()
    }
}


pub fn value_is_nil(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Returns whether a value is nil */

    let args = evaluator.resolve_globals(args);

    match args.as_slice() {
        [continuation, value] => {
            let result = match &**value {
                Value::Nil => true,
                _ => false
            };

            Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
        },

        _ => new_error!("Liszp: function 'nil?' takes exactly one argument").into()
    }
}


pub fn value_is_name(args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
    /* Returns whether a value is a name */

    match args.as_slice() {
        [continuation, value] => {
            let result = match &**value {
                Value::Name(_) => true,
                _ => false
            };

            Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
        },

        _ => new_error!("Liszp: function 'name?' takes exactly one argument").into()
    }
}


pub fn value_is_str(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Returns whether a value is a str */

    let args = evaluator.resolve_globals(args);

    match args.as_slice() {
        [continuation, value] => {
            let result = match &**value {
                Value::String(_) => true,
                _ => false
            };

            Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
        },

        _ => new_error!("Liszp: function 'str?' takes exactly one argument").into()
    }
}
