use crate::error::Error;
use crate::eval::Evaluator;
use crate::new_error;
use crate::refcount_list;
use crate::value::Value;
use std::rc::Rc;


pub fn car(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Gets the car of a cons pair */

    match args.as_slice() {
        [continuation, xs] => {
            let resolved = evaluator.resolve(xs)?;

            let car = match &*resolved {
                Value::Cons { car, .. } => car,
                _ => return new_error!("Liszp: function 'cons' expected to receive cons pair").into()
            };

            Ok(refcount_list![ continuation, car ])
        },

        _ => new_error!("Liszp: function 'car' takes 1 argument").into()
    }
}


pub fn cdr(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Gets the cdr of a cons pair */

    match args.as_slice() {
        [continuation, xs] => {
            let resolved = evaluator.resolve(xs)?;

            let cdr = match &*resolved {
                Value::Cons { cdr, .. } => cdr,
                _ => return new_error!("Liszp: function 'cons' expected to receive cons pair").into()
            };


            Ok(refcount_list![ continuation, cdr ])
        },

        _ => new_error!("Liszp: function 'cdr' takes 1 argument").into()
    }
}


pub fn cons(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Creates a cons pair */

    match args.as_slice() {
        [continuation, car, cdr] => {
            let cons_pair = Value::Cons {
                car: evaluator.resolve(car)?,
                cdr: evaluator.resolve(cdr)?
            };

            Ok(refcount_list![ continuation.clone(), cons_pair.rc() ])
        }

        _ => new_error!("Liszp: function 'cons' expected 2 arguments").into()
    }
}


pub fn eval_quoted(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Evaluates a quoted value */

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

    if args.len() != 3 {
        return new_error!("Liszp: if expression has syntax (if <condition> <true case> <false case>)").into();
    }

    let cond = evaluator.resolve(&args[0])?;
    let true_case = evaluator.resolve(&args[1])?;
    let false_case = evaluator.resolve(&args[2])?;

    if let Value::Bool(b) = &*cond {
        if *b {
            Ok(true_case)
        } else {
            Ok(false_case)
        }
    } else {
        new_error!("if expression expected a boolean condition").into()
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

    if args.len() != 2 {
        return new_error!("Function print{} takes 1 argument only", if newline { "ln" } else { "" }).into();
    }

    let continuation = &args[0];
    let value = evaluator.resolve(&args[1])?;

    if newline {
        println!("{}", value);
    } else {
        print!("{}", value);
    }

    Ok(refcount_list![ continuation.clone(), value])
}


pub fn quote_value(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Quotes a value */

    match args.as_slice() {
        [continuation, value] => Ok(refcount_list![ continuation, value ]),

        _ => new_error!("Liszp: function 'quote' takes exactly one value").into()
    }
}


pub fn values_are_equal(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Compares two values */

    match args.as_slice() {
        [continuation, x, y] => {
            let result = Value::Bool(evaluator.resolve(x)? == evaluator.resolve(y)?).rc();

            Ok(refcount_list![ continuation, &result ])
        },

        _ => new_error!("Liszp: Function 'equals?' takes exactly 2 parameters").into()
    }
}


pub fn value_is_bool(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Returns whether a value is a bool */

    match args.as_slice() {
        [continuation, value] => {
            let result = match &*evaluator.resolve(value)? {
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

    match args.as_slice() {
        [continuation, value] => {
            let result = match &*evaluator.resolve(value)? {
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

    match args.as_slice() {
        [continuation, value] => {
            let result = match &*evaluator.resolve(value)? {
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

    match args.as_slice() {
        [continuation, value] => {
            let result = match &*evaluator.resolve(value)? {
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

    match args.as_slice() {
        [continuation, value] => {
            let result = match &*evaluator.resolve(value)? {
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

    match args.as_slice() {
        [continuation, value] => {
            let result = match &*evaluator.resolve(value)? {
                Value::String(_) => true,
                _ => false
            };

            Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
        },

        _ => new_error!("Liszp: function 'str?' takes exactly one argument").into()
    }
}
