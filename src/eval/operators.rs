use crate::eval::eval_main::{ Env, resolve_value };
use crate::read::Value;
use crate::refcount_list;
use crate::remove_amp;

use std::collections::LinkedList;
use std::rc::Rc;

use itertools::Itertools;
use rug;

/* Arithmetic */

fn float_arithmetic(op: String, numbers: LinkedList<&Rc<Value>>) -> Value {
    /* Evaluates an arithmetic expression where one or more of the parameters are floats */

    let mut result = match &***numbers.front().unwrap() {
        Value::Float(f) => f.clone(),
        Value::Integer(i) => rug::Float::with_val(53, i),
        _ => panic!("This really shouldn't happen")
    };

    macro_rules! reduce_stmt {
        { $action:tt } => {
            for n in numbers.iter().dropping(1) {
                match &***n {
                    Value::Float(f) => { result $action f },
                    Value::Integer(i) => { result $action i },
                    _ => panic!("This should never happen")
                }
            }
        }
    }

    match &op[..] {
        "+&" => reduce_stmt!(+=),
        "-&" => reduce_stmt!(-=),
        "*&" => reduce_stmt!(*=),
        "/&" => reduce_stmt!(/=),
         _   => {
            for n in numbers.iter().dropping(1) {
                match &***n {
                    Value::Float(f) => result %= f,
                    Value::Integer(_) => panic!("cannot take integer modulo of float"),
                    _ => panic!("This should never happen")
                }
            }
         }
    }

    return if &op[..] == "-&" && numbers.len() == 1 {
        Value::Float(-result)
    } else {
        Value::Float(result)
    };
}

fn integer_arithmetic(op: String, numbers: LinkedList<&Rc<Value>>) -> Value {
    /* Evaluates an arithmetic expression consisting of all integers */

    let mut result = match &***numbers.front().unwrap() {
        Value::Integer(i) => i.clone(),
        _ => panic!("This absolutely shouldn't happen")
    };

    macro_rules! reduce_stmt {
        { $action:tt } => {
            for n in numbers.iter().dropping(1) {
                match &***n {
                    Value::Integer(i) => { result $action i },
                    _ => panic!("This should never happen")
                }
            }
        }
    }

    match &op[..] {
        "+&" => reduce_stmt!(+=),
        "-&" => reduce_stmt!(-=),
        "*&" => reduce_stmt!(*=),
        "/&" => reduce_stmt!(/=),
        _    => reduce_stmt!(%=)
    }

    return if &op[..] == "-&" && numbers.len() == 1 {
        Value::Integer(-result)
    } else {
        Value::Integer(result)
    };
}

pub (in crate::eval) fn arithmetic(op: String, parameters: Rc<Value>, env: &Env) -> Rc<Value> {
    /* Evaluates an arithmetic expression */

    let mut numbers = LinkedList::new();
    let mut floats = false;

    let mut parameter_list = parameters.to_list()
                                                      .expect(&format!("Expected {} function to have args", op)[..]);

    let continuation = if parameter_list.len() > 1 {
        let c = Rc::clone(parameter_list.front().unwrap());
        parameter_list = parameter_list.split_off(1);

        c
    } else {
        panic!("Function '{}' supplied with no arguments", op);
    };

    for param in parameter_list.iter() {
        match &**param {
            Value::Float(_) => {
                floats = true;
                numbers.push_back(param);
            },

            Value::Integer(_) => {
                numbers.push_back(param)
            },

            Value::Name(_) => {
                numbers.push_back(crate::eval::eval_main::resolve_value(param, env))
            },

            _ => panic!("Expected number literal or variable containing number in '{}' expression", op)
        }
    }

    let result = if floats {
        float_arithmetic(op, numbers)
    } else {
        integer_arithmetic(op, numbers)
    };

    return refcount_list![ continuation, result.refcounted() ];
}

/* Comparison */

fn integer_comparison(op: String, x: &rug::Integer, y: &rug::Integer) -> Rc<Value> {
    /* Compares two integer values */

    let result = match &op[..] {
        "<&"  => x < y,
        ">&"  => x > y,
        "<=&" => x <= y,
        ">=&" => x >= y,
        "==&" => x == y,
        _     => x != y
    };

    return Value::Bool(result).refcounted();
}

fn float_comparison(op: String, x: rug::Float, y: rug::Float) -> Rc<Value> {
    /* Compares two floating point values */

    let result = match &op[..] {
        "<&"  => x < y,
        ">&"  => x > y,
        "<=&" => x <= y,
        ">=&" => x >= y,
        "==&" => x == y,
        _     => x != y
    };

    return Value::Bool(result).refcounted();
}

pub (in crate::eval) fn comparison(op: String, parameters: Rc<Value>, env: &Env) -> Rc<Value> {
    /* Compare two numeric values */

    crate::unroll_parameters!(
        parameters,
        &format!("Liszp: expected syntax ({} <value> <value>)", op)[..],
        true ;
        k, a, b
    );

    let res1 = resolve_value(a, env);
    let res2 = resolve_value(b, env);

    let result = match (&**res1, &**res2) {
        (Value::Integer(x), Value::Integer(y)) => {
            integer_comparison(op, x, y)
        },

        (Value::Integer(x), Value::Float(y)) => {
            let xval = rug::Float::with_val(53, 0) + x;
            let yval = y.clone();

            float_comparison(op, xval, yval)
        },

        (Value::Float(x), Value::Integer(y)) => {
            let xval = x.clone();
            let yval = rug::Float::with_val(53, 0) + y;

            float_comparison(op, xval, yval)
        },

        (Value::Float(x), Value::Float(y)) => {
            float_comparison(op, x.clone(), y.clone())
        },

        _ => panic!("Expected numeric arguments for '{}' function", remove_amp!(op))
    };

    return refcount_list![ Rc::clone(k), result ];
}

/* Boolean */

pub (in crate::eval) fn boolean(op: String, parameters: Rc<Value>, env: &Env) -> Rc<Value> {
    /* Boolean logic function (and, or, not, etc) */

    let parameter_list = parameters.to_list()
                                   .expect("Expected syntax (not <value>)");

    if &op[..] == "not&" && parameter_list.len() != 2 {
        panic!("Function 'not' expected 1 argument, recieved {}", parameter_list.len() - 2);
    } else if &op[..] != "not&" && parameter_list.len() != 3 {
        println!("{}", op);
        panic!("Function '{}' expected 2 arguments, recieved {}", remove_amp!(op), parameter_list.len() - 2);
    }

    let mut plist_iter = parameter_list.iter();
    let k = plist_iter.next().unwrap();
    let x = if let Value::Bool(b) = **resolve_value(plist_iter.next().unwrap(), env) {
        b
    } else {
        panic!("Function '{}' expected boolean arguments", remove_amp!(op));
    };

    let y = if let Value::Bool(b) = **resolve_value(plist_iter.next().unwrap_or(&Value::Bool(true).refcounted()), env) {
        b
    } else {
        panic!("Function '{}' expected boolean arguments", remove_amp!(op));
    };

    let result = match &op[..] {
        "not&" => !x,
        "and&" => x && y,
        "or&"  => x || y,
        _      => x ^ y     // xor
    };

    return refcount_list![ Rc::clone(k), Value::Bool(result).refcounted() ];
}
