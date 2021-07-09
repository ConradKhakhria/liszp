use crate::parse::Value;
use crate::eval::eval_main::{Env, resolve_value};

use std::collections::LinkedList;
use std::rc::Rc;

pub (in crate::eval) fn define_value(parameters: &Rc<Value>, env: &mut Env) -> Rc<Value> {
    /* Adds a value to the global namespace */

    let parameter_list = parameters.to_list()
                                   .expect("Liszp: Expected def expression with syntax (def <name> <value>)");

    let mut p_iter = parameter_list.iter();

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

pub (in crate::eval) fn print_value(parameters: &Rc<Value>, env: &mut Env, name: String) -> Rc<Value> {
    /* Prints a value and then returns it */

    let parameter_list = parameters.to_list()
                                   .expect(&format!("Function {} expected an argument", name)[..]);

    if parameter_list.len() != 2 {
        panic!("Function '{}' expected 1 argument but received {}", name, parameter_list.len() - 1);
    }

    let mut plist_iter = parameter_list.iter();

    let k = plist_iter.next().unwrap();
    let v = resolve_value(plist_iter.next().unwrap(), env);

    if &name[..] == "println&" {
        println!("{}", v);
    } else {
        print!("{}", v);
    }

    return Rc::new(Value::Cons {
        car: Rc::clone(k),
        cdr: Rc::new(Value::Cons {
            car: Rc::clone(&v),
            cdr: Rc::new(Value::Nil)
        })
    })
}

pub (in crate::eval) fn if_expr(parameters: &Rc<Value>, env: &mut Env) -> Rc<Value> {
    /* Evaluates an if expression */

    let parameter_list = parameters.to_list()
                                   .expect("Expected syntax (if <cond> <true case> <false case>");

    if parameter_list.len() != 3 {
        panic!("if expression expected 3 arguments, received {}", parameter_list.len());
    }

    let mut plist_iter = parameter_list.iter();
    
    let c = plist_iter.next().unwrap();
    let t = plist_iter.next().unwrap();
    let f = plist_iter.next().unwrap();

    let cond = if let Value::Bool(b) = *resolve_value(c, env) {
        b
    } else {
        panic!("Expected boolean condition in if expr");
    };

    return if cond {
        Rc::clone(t)
    } else {
        Rc::clone(f)
    };
}


