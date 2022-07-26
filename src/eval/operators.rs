/* A module for arithmetic, logic, and comparative operators */

use crate::error::Error;
use crate::eval::Evaluator;
use crate::new_error;
use crate::refcount_list;
use crate::value::Value;
use itertools::Itertools;
use rug;
use std::rc::Rc;


/* Arithmetic */


pub fn arithmetic_expression(op: &String, args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Computes an arithmetic expression */

    if args.len() < 2 {
        return new_error!("Liszp: '{}' expression takes at least 1 argument", op).into();
    }

    let mut numbers = Vec::with_capacity(args.len());
    let continuation = &args[0];
    let mut result_is_float = false;

    for arg in args.iter().dropping(1) {
        let arg = evaluator.resolve(arg)?;

        match &*arg {
            Value::Float(_) => {
                result_is_float = true;
                numbers.push(arg);
            },

            Value::Integer(_) => numbers.push(arg),

            _ => return new_error!("Liszp: '{}' expression takes numeric arguments", &op[1..]).into()
        }
    }

    let result = if result_is_float {
        float_arithmetic(op, &numbers)
    } else {
        integer_arithmetic(op, &numbers)
    };

    Ok(refcount_list![ continuation.clone(), result ])
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
        "&+" => reduce_over_operation!(+=),
        "&-" => reduce_over_operation!(-=),
        "&*" => reduce_over_operation!(*=),
        "&/" => reduce_over_operation!(/=),
        _    => unreachable!()
    }

    if op == "&-" && args.len() == 1 {
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
        "&+" => reduce_over_operation!(+=),
        "&-" => reduce_over_operation!(-=),
        "&*" => reduce_over_operation!(*=),
        "&/" => reduce_over_operation!(/=),
        _    => unreachable!()
    }

    if op == "&-" && args.len() == 1 {
        Value::Integer(-result).rc()
    } else {
        Value::Integer(result).rc()
    }
}


pub fn modulo(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Takes the modulus of a number */

    match args.as_slice() {
        [continuation, dividend, divisor] => {
            let dividend = evaluator.resolve(dividend)?;
            let divisor = evaluator.resolve(divisor)?;

            let result = match (&*dividend, &*divisor) {
                (Value::Float(x), Value::Float(y)) => Value::Float(x.clone() % y.clone()).rc(),

                (Value::Float(_), Value::Integer(_)) => return new_error!("Liszp: Cannot take the integer modulo of a float").into(),

                (Value::Integer(x), Value::Integer(y)) => Value::Integer(x.clone() % y.clone()).rc(),

                _ => unreachable!()
            };

            Ok(refcount_list![ continuation, &result ])
        },

        _ => new_error!("Liszp: modulo expressions take exactly 2 arguments").into()
    }
}


/* Logic */

pub fn binary_logical_operation(op: &String, args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Evaluates a binary logical operation */

    match args.as_slice() {
        [continuation, x, y] => {
            let x = match &*evaluator.resolve(x)? {
                Value::Bool(b) => *b,
                _ => return new_error!("Liszp: {} expressions take boolean arguments", &op[1..]).into()
            };

            let y = match &*evaluator.resolve(y)? {
                Value::Bool(b) => *b,
                _ => return new_error!("Liszp: {} expressions take boolean arguments", &op[1..]).into()
            };

            let result = match op.as_str() {
                "&and" => x && y,
                "&or"  => x || y,
                "&xor" => x ^ y,
                _      => unreachable!()
            };

            Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
        }

        _ => new_error!("Liszp: {} expressions take exactly 2 arguments", &op[1..]).into()
    }
}


pub fn logical_negation(args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Performs a logical not operation */

    match args.as_slice() {
        [continuation, x] => {
            let x = match &*evaluator.resolve(x)? {
                Value::Bool(b) => *b,
                _ => return new_error!("Liszp: not expressions take a boolean argument").into()
            };

            let result = Value::Bool(!x).rc();

            Ok(refcount_list![ continuation, &result ])
        }

        _ => new_error!("Liszp: not expressions take exactly 1 argument").into()
    }
}


/* Comparison */

pub fn comparison(op: &String, args: &Vec<Rc<Value>>, evaluator: &Evaluator) -> Result<Rc<Value>, Error> {
    /* Compares two values */

    match args.as_slice() {
        [continuation, x, y] => {
            let x = evaluator.resolve(x)?;
            let y = evaluator.resolve(y)?;

            let result = match (&*x, &*y) {
                (Value::Integer(x), Value::Integer(y)) => {
                    integer_comparison(op, x, y)
                }

                (Value::Float(x), Value::Integer(y)) => {
                    let y = rug::Float::with_val(53, y);

                    float_comparison(op, x, &y)
                }

                (Value::Integer(x), Value::Float(y)) => {
                    let x = rug::Float::with_val(53, x);

                    float_comparison(op, &x, y)
                }

                (Value::Float(x), Value::Float(y)) => {
                    float_comparison(op, x, y)
                }

                _ => return new_error!("Liszp: {} expressions take two numeric values", &op[1..]).into()
            };

            Ok(refcount_list![ continuation, &result ])
        }

        _ => new_error!("Liszp: {} expressions take exactly 2 values", &op[1..]).into()
    }
}


fn float_comparison(op: &String, x: &rug::Float, y: &rug::Float) -> Rc<Value> {
    /* Compares two floats */

    let result = match op.as_str() {
        "&==" => x == y,
        "&!=" => x != y,
        "&<"  => x < y,
        "&>"  => x > y,
        "&<=" => x <= y,
        "&>=" => x >= y,
        _     => unreachable!()
    };

    Value::Bool(result).rc()
}


fn integer_comparison(op: &String, x: &rug::Integer, y: &rug::Integer) -> Rc<Value> {
    /* Compares two integers */

    let result = match op.as_str() {
        "&==" => x == y,
        "&!=" => x != y,
        "&<"  => x < y,
        "&>"  => x > y,
        "&<=" => x <= y,
        "&>=" => x >= y,
        _     => unreachable!()
    };

    Value::Bool(result).rc()
}
