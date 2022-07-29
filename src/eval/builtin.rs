use crate::error::Error;
use crate::eval::Evaluator;
use crate::new_error;
use crate::value::Value;
use std::rc::Rc;


pub fn car(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Gets the car of a cons pair */

    match args.as_slice() {
        [cons] => {
            match &**cons {
                Value::Cons { car, .. } => evaluator.eval(car),
                _ => new_error!("Liszp: function 'cons' expected to receive cons pair").into()
            }
        },

        _ => new_error!("Liszp: function 'car' takes 1 argument").into()
    }
}


pub fn cdr(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Gets the cdr of a cons pair */

    match args.as_slice() {
        [cons] => {
            match &**cons {
                Value::Cons { cdr, .. } => evaluator.eval(cdr),
                _ => new_error!("Liszp: function 'cons' expected to receive cons pair").into()
            }
        },

        _ => new_error!("Liszp: function 'cdr' takes 1 argument").into()
    }
}


pub fn cons(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Creates a cons pair */

    match args.as_slice() {
        [car, cdr] => {
            let car = evaluator.eval(car)?;
            let cdr = evaluator.eval(cdr)?;

            Ok(Value::cons(&car, &cdr).rc())
        },

        _ => new_error!("Liszp: function 'cons' expected 2 arguments").into()
    }
}


pub fn eval_quoted(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Evaluates a quoted value */

    match args.as_slice() {
        [quoted_value] => evaluator.eval(quoted_value),
        _ => new_error!("Liszp: function 'quote' takes exactly one argument").into()
    }
}


pub fn if_expr(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Evaluates an if expression */

    match args.as_slice() {
        [cond, true_case, false_case] => {
            if let Value::Bool(b) = &*evaluator.eval(cond)? {
                if *b {
                    evaluator.eval(true_case)
                } else {
                    evaluator.eval(false_case)
                }
            } else {
                new_error!("if expression expected a boolean condition").into()
            }
        },

        _ => new_error!("Liszp: if expression has syntax (if <condition> <true case> <false case>)").into()
    }
}


pub fn panic(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Panics */

    match args.as_slice() {
        [msg] => panic!("{}", evaluator.eval(msg)?),
        _ => new_error!("Liszp: expected syntax (panic <message>)").into()
    }
}


pub fn print_value(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator, newline: bool) -> Result<Rc<Value>, Error> {
    /* Prints a value, optionally with a newline */

    match args.as_slice() {
        [value] => {
            let evaluated = evaluator.eval(value)?;

            if newline {
                println!("{}", &evaluated);
            } else {
                print!("{}", &evaluated);
            }
        
            Ok(evaluated)
        },

        _ => new_error!("Function print{} takes 1 argument only", if newline { "ln" } else { "" }).into()
    }
}


pub fn quote_value(args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
    /* Quotes a value */

    match args.as_slice() {
        [value] => Ok(value.clone()),
        _ => new_error!("Liszp: function 'quote' takes exactly one value").into()
    }
}


pub fn values_are_equal(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Compares two values */

    match args.as_slice() {
        [x, y] => {
            let x = evaluator.eval(x)?;
            let y = evaluator.eval(y)?;

            Ok(Value::Bool(x == y).rc())
        },

        _ => new_error!("Liszp: Function 'equals?' takes exactly 2 parameters").into()
    }
}


pub fn value_is_bool(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Returns whether a value is a bool */

    match args.as_slice() {
        [value] => {
            let res = match &*evaluator.eval(value)? {
                Value::Bool(_) => true,
                _ => false
            };

            Ok(Value::Bool(res).rc())
        },

        _ => new_error!("Liszp: function 'bool?' takes exactly one argument").into()
    }
}


pub fn value_is_cons(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Returns whether a value is a cons pair */

    match args.as_slice() {
        [value] => {
            let res = match &*evaluator.eval(value)? {
                Value::Cons {..} => true,
                _ => false
            };

            Ok(Value::Bool(res).rc())
        },

        _ => new_error!("Liszp: function 'cons?' takes exactly one argument").into()
    }
}


pub fn value_is_float(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {   
    /* Returns whether a value is a float */

    match args.as_slice() {
        [value] => {
            let res = match &*evaluator.eval(value)? {
                Value::Float(_) => true,
                _ => false
            };

            Ok(Value::Bool(res).rc())
        },

        _ => new_error!("Liszp: function 'float?' takes exactly one argument").into()
    }
}


pub fn value_is_int(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Returns whether a value is an int */

    match args.as_slice() {
        [value] => {
            let res = match &*evaluator.eval(value)? {
                Value::Integer(_) => true,
                _ => false
            };

            Ok(Value::Bool(res).rc())
        },

        _ => new_error!("Liszp: function 'int?' takes exactly one argument").into()
    }
}


pub fn value_is_nil(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Returns whether a value is nil */

    match args.as_slice() {
        [value] => {
            let res = match &*evaluator.eval(value)? {
                Value::Nil => true,
                _ => false
            };

            Ok(Value::Bool(res).rc())
        },

        _ => new_error!("Liszp: function 'nil?' takes exactly one argument").into()
    }
}


pub fn value_is_name(args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
    /* Returns whether a value is a name */

    match args.as_slice() {
        [value] => {
            let result = match &**value {
                Value::Name(_) => true,
                _ => false
            };

            Ok(Value::Bool(result).rc())
        },

        _ => new_error!("Liszp: function 'name?' takes exactly one argument").into()
    }
}


pub fn value_is_str(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Returns whether a value is a str */

    match args.as_slice() {
        [value] => {
            let res = match &*evaluator.eval(value)? {
                Value::String(_) => true,
                _ => false
            };

            Ok(Value::Bool(res).rc())
        },

        _ => new_error!("Liszp: function 'str?' takes exactly one argument").into()
    }
}
