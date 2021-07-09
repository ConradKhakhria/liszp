use crate::parse::Value;
use crate::eval::eval_main::{Env, resolve_value};

use std::collections::LinkedList;
use std::rc::Rc;

pub (in crate::eval) fn define_value(parameters: &Rc<Value>, env: &mut Env) -> Rc<Value> {
    /* Adds a value to the global namespace */

    crate::unroll_parameters!(
        parameters,
        "Liszp: function 'def' expected syntax (def <name> <value>)",
        false;
        name_value, body_value
    );

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

    crate::unroll_parameters!(
        parameters,
        &format!("Liszp: function '{}' expected syntax ({} <value>)", name, name)[..],
        true ;
        k, v
    );

    let value = resolve_value(v, env);

    if &name[..] == "println&" {
        println!("{}", *value);
    } else {
        print!("{}", *value);
    }

    return crate::refcount_list![k, &value];
}

pub (in crate::eval) fn if_expr(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Evaluates an if expression */

    crate::unroll_parameters!(
        parameters,
        "Liszp: function 'if' expected syntax (if <cond> <true case> <false case>)",
        false ;
        c, t, f
    );

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

/* Cons functions */

pub (in crate::eval) fn cons(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Creates a cons pair */

    crate::unroll_parameters!(
        parameters,
        "Liszp: expected syntax (cons <value> <value>)",
        true ;
        k, a, b
    );

    let resolved = resolve_value(b, env);
    
    let cdr = if let Value::Quote(v) = &*resolved {
        &v
    } else {
        b
    };

    let quote = Value::Quote(
        Rc::new(Value::Cons {
            car: Rc::clone(&resolve_value(a, env)),
            cdr: Rc::clone(cdr)
        })
    );

    return crate::value_list![ &**k, &quote ];
}
