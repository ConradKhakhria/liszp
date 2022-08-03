/* A module for arithmetic, logic, and comparative operators */

use crate::error::Error;
use crate::eval::Evaluator;
use crate::new_error;
use crate::value::Value;
use itertools::Itertools;
use rug;
use std::rc::Rc;


/* Arithmetic */


pub fn arithmetic_expression(op: &String, args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Computes an arithmetic expression */

    if args.len() < 2 {
        return new_error!("Liszp: '{}' expression takes at least 1 argument", op).into();
    }

    let mut numbers = Vec::with_capacity(args.len());
    let mut result_is_float = false;

    for arg in args.iter() {
        let arg = evaluator.eval(arg)?;

        match &*arg {
            Value::Float(_) => {
                result_is_float = true;
                numbers.push(arg.clone());
            },

            Value::Integer(_) => numbers.push(arg.clone()),

            _ => return new_error!("Liszp: '{}' expression takes numeric arguments", op).into()
        }
    }

    if result_is_float {
        Ok(float_arithmetic(op, &numbers))
    } else {
        Ok(integer_arithmetic(op, &numbers))
    }
}


fn float_arithmetic(op: &String, args: &Vec<Rc<Value>>) -> Rc<Value> {
    /* Evaluates an arithmetic expression of floats */

    let mut result = match &*args[0] {
        Value::Float(f) => f.clone(),
        Value::Integer(i) => rug::Float::with_val(53, i),
        _ => unreachable!()
    };

    macro_rules! reduce_over_operation {
        { $action:tt } => {
            for arg in args.iter().dropping(1) {
                match &**arg {
                    Value::Float(f) => { result $action f },
                    Value::Integer(i) => { result $action i },
                    _ => unreachable!()
                }
            }
        }
    }

    match op.as_str() {
        "+" => reduce_over_operation!(+=),
        "-" => reduce_over_operation!(-=),
        "*" => reduce_over_operation!(*=),
        "/" => reduce_over_operation!(/=),
        _    => unreachable!()
    }

    if op == "-" && args.len() == 1 {
        Value::Float(-result).rc()
    } else {
        Value::Float(result).rc()
    }
}


fn integer_arithmetic(op: &String, args: &Vec<Rc<Value>>) -> Rc<Value> {
    /* Evaluates an arithmetic expression of integers */

    let mut result = match &*args[0] {
        Value::Integer(i) => i.clone(),
        _ => unreachable!()
    };

    macro_rules! reduce_over_operation {
        { $action:tt } => {
            for arg in args.iter().dropping(1) {
                match &**arg {
                    Value::Integer(i) => { result $action i },
                    _ => unreachable!()
                }
            }
        }
    }

    match op.as_str() {
        "+" => reduce_over_operation!(+=),
        "-" => reduce_over_operation!(-=),
        "*" => reduce_over_operation!(*=),
        "/" => reduce_over_operation!(/=),
        _    => unreachable!()
    }

    if op == "-" && args.len() == 1 {
        Value::Integer(-result).rc()
    } else {
        Value::Integer(result).rc()
    }
}


pub fn modulo(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Takes the modulus of a number */

    match args.as_slice() {
        [dividend, divisor] => {
            match (&*evaluator.eval(dividend)?, &*evaluator.eval(divisor)?) {
                (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.clone() % y.clone()).rc()),

                (Value::Float(_), Value::Integer(_)) => new_error!("cannot take the integer modulo of a float").into(),

                (Value::Integer(x), Value::Integer(y)) => Ok(Value::Integer(x.clone() % y.clone()).rc()),

                _ => unreachable!()
            }
        },

        _ => new_error!("modulo expressions take exactly 2 arguments").into()
    }
}


/* Logic */

pub fn logical_operation(op: &String, args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Evaluates a binary logical operation */

    if args.len() < 2 {
        return new_error!("cannot apply '{}' expression to only {} arg", op, args.len()).into();
    }

    let mut boolean_result = match &*evaluator.eval(&args[0])? {
        Value::Bool(b) => *b,
        _ => return new_error!("{} expressions take boolean arguments", op).into()
    };

    for arg in args[1..].iter() {
        let boolean = match &*evaluator.eval(arg)? {
            Value::Bool(b) => *b,
            _ => return new_error!("{} expressions take boolean arguments", op).into()
        };

        match op.as_str() {
            "and" => boolean_result = boolean_result && boolean,
            "or"  => boolean_result = boolean_result || boolean,
            "xor" => boolean_result = boolean_result ^  boolean,
            _ => unreachable!()
        }
    }

    Ok(Value::Bool(boolean_result).rc())
}


pub fn logical_negation(args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Performs a logical not operation */

    match args.as_slice() {
        [x] => {
            match &*evaluator.eval(x)? {
                Value::Bool(b) => Ok(Value::Bool(!*b).rc()),
                _ => new_error!("'not' expressions take a boolean argument").into()
            }
        }

        _ => new_error!("'not' expressions take exactly 1 argument").into()
    }
}


/* Comparison */


pub fn comparison(op: &String, args: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Compares two values */

    match args.as_slice() {
        [x, y] => {
            match (&*evaluator.eval(x)?, &*evaluator.eval(y)?) {
                (Value::Integer(x), Value::Integer(y)) => {
                    Ok(integer_comparison(op, x, y))
                }

                (Value::Float(x), Value::Integer(y)) => {
                    let y = rug::Float::with_val(53, y);

                    Ok(float_comparison(op, x, &y))
                }

                (Value::Integer(x), Value::Float(y)) => {
                    let x = rug::Float::with_val(53, x);

                    Ok(float_comparison(op, &x, y))
                }

                (Value::Float(x), Value::Float(y)) => {
                    Ok(float_comparison(op, x, y))
                }

                _ => new_error!("{} expressions take two numeric values", op).into()
            }
        }

        _ => new_error!("{} expressions take exactly 2 values", op).into()
    }
}


fn float_comparison(op: &String, x: &rug::Float, y: &rug::Float) -> Rc<Value> {
    /* Compares two floats */

    let result = match op.as_str() {
        "==" => x == y,
        "!=" => x != y,
        "<"  => x < y,
        ">"  => x > y,
        "<=" => x <= y,
        ">=" => x >= y,
        _     => unreachable!()
    };

    Value::Bool(result).rc()
}


fn integer_comparison(op: &String, x: &rug::Integer, y: &rug::Integer) -> Rc<Value> {
    /* Compares two integers */

    let result = match op.as_str() {
        "==" => x == y,
        "!=" => x != y,
        "<"  => x < y,
        ">"  => x > y,
        "<=" => x <= y,
        ">=" => x >= y,
        _     => unreachable!()
    };

    Value::Bool(result).rc()
}
