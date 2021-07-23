use crate::eval::eval_main::{ Env, resolve_value };
use crate::read::Value;
use crate::remove_amp;

use std::collections::LinkedList;
use std::rc::Rc;

use itertools::Itertools;
use rug;

/* Arithmetic */

fn float_arithmetic(op: String, numbers: LinkedList<Rc<Value>>, env: &Env) -> Value {
    /* Evaluates an arithmetic expression where one or more of the parameters are floats */

    fn make_float(val: &Rc<Value>, env: &Env) -> rug::Float {
        return match &*resolve_value(val, env) {
            Value::Float(f) => f.clone(),
            Value::Integer(i) => rug::Float::with_val(53, 0) + i, // ugly
            _ => panic!("This shouldn't happen ever")
        };
    }

    let mut result = make_float(numbers.front().unwrap(), env);

    for n in numbers.iter().dropping(1) {
        let num = make_float(n, env);

        match &op[..] {
            "+&" => result += num,
            "-&" => result -= num,
            "*&" => result *= num,
            "/&" => result /= num,
            _    => result %= num
        }
    }

    return if &op[..] == "-&" && numbers.len() == 1 {
        Value::Float(-result)
    } else {
        Value::Float(result)
    };
}

fn integer_arithmetic(op: String, numbers: LinkedList<Rc<Value>>, env: &Env) -> Value {
    /* Evaluates an arithmetic expression consisting of all integers */

    fn make_integer(val: &Rc<Value>, env: &Env) -> rug::Integer {
        return match &*resolve_value(val, env) {
            Value::Integer(i) => i.clone(),
            _ => panic!("This absolutely shouldn't happen")
        };
    }

    let mut result = make_integer(numbers.front().unwrap(), env);

    for n in numbers.iter().dropping(1) {
        let num = make_integer(n, env);

        match &op[..] {
            "+&" => result += num,
            "-&" => result -= num,
            "*&" => result *= num,
            "/&" => result /= num,
            _    => result %= num
        }
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
                numbers.push_back(Rc::clone(param));
            },

            Value::Integer(_) => {
                numbers.push_back(Rc::clone(param))
            },

            Value::Name(_) => {
                numbers.push_back(Rc::clone(&crate::eval::eval_main::resolve_value(param, env)))
            },

            _ => panic!("Expected number literal or variable containing number in '{}' expression", op)
        }
    }

    let result = if floats {
        float_arithmetic(op, numbers, env)
    } else {
        integer_arithmetic(op, numbers, env)
    };

    return Rc::new(Value::Cons {
        car: continuation,
        cdr: Rc::new(Value::Cons {
            car: result.refcounted(),
            cdr: Value::Nil.refcounted()
        })
    });
}

/* Comparison */

fn integer_comparison(op: String, x: rug::Integer, y: rug::Integer) -> Rc<Value> {
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

    let result = match (&*res1, &*res2) {
        (Value::Integer(x), Value::Integer(y)) => {
            integer_comparison(op, x.clone(), y.clone())
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

    return Rc::new(Value::Cons {
        car: Rc::clone(k),
        cdr: Rc::new(Value::Cons {
            car: result,
            cdr: Value::Nil.refcounted()
        })
    })    
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
    let x = if let Value::Bool(b) = *resolve_value(plist_iter.next().unwrap(), env) {
        b
    } else {
        panic!("Function '{}' expected boolean arguments", remove_amp!(op));
    };

    let y = if let Value::Bool(b) = *resolve_value(plist_iter.next().unwrap_or(&Value::Bool(true).refcounted()), env) {
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

    return Rc::new(Value::Cons {
        car: Rc::clone(k),
        cdr: Rc::new(Value::Cons {
            car: Value::Bool(result).refcounted(),
            cdr: Value::Nil.refcounted()
        })
    });
}
