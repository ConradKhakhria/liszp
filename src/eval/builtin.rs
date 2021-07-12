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

pub (in crate::eval) fn compare_values(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Compares two values of any type */

    crate::unroll_parameters! {
        parameters,
        "Liszp: expected syntax (equals? <value> <value>)",
        true ;
        k, x, y
    };

    let result = &Rc::new(Value::Bool(
        resolve_value(x, env).eq(&resolve_value(y, env))
    ));

    return crate::refcount_list![ k,  result ];
}

pub (in crate::eval) fn get_length(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Gets the length of a value */

    crate::unroll_parameters! {
        parameters,
        "Liszp: expected syntax (len <value>)",
        true ;
        k, xs
    };

    let result = &Rc::new(Value::Integer(
        rug::Integer::from(resolve_value(xs, env).len())
    ));

    return crate::refcount_list![ k, result ];
}

pub (in crate::eval) fn quote(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Quotes a value */

    crate::unroll_parameters!(
        parameters,
        "Liszp: expected syntax (quote <value>)",
        true ;
        k, x
    );

    let value = Rc::new(Value::Quote(Rc::clone(&resolve_value(x, env))));

    return crate::refcount_list![ k, &value ];
}

pub (in crate::eval) fn unquote(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Unquotes a value */

    crate::unroll_parameters!(
        parameters,
        "Liszp: expected syntax (unquote <value>)",
        true ;
        k, x
    );

    if let Value::Quote(v) = &*resolve_value(x, env) {
        let cloned = Rc::clone(v);

        return crate::refcount_list![ k, &cloned ];
    } else {
        panic!("Liszp: attempt to unquote a value that isn't quoted");
    }
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

pub (in crate::eval) fn car(parameters: &Rc<Value>, env: &Env, name: String) -> Rc<Value> {
    /* Takes car of a cons pair */

    crate::unroll_parameters!(
        parameters,
        &format!("Liszp: expected syntax ({} <cons pair>)", name)[..],
        true ;
        k, x
    );

    let mut resolved = resolve_value(x, env);

    if let Value::Quote(cons) = &*resolved {
        resolved = Rc::clone(cons);
    }

    let car = if let Value::Cons { car, .. } = &*resolved {
        Rc::clone(car)
    } else {
        panic!("Liszp: function {} expected to receive a cons pair", name);
    };

    return crate::refcount_list![ k, &car ];
}

pub (in crate::eval) fn cdr(parameters: &Rc<Value>, env: &Env, name: String) -> Rc<Value> {
    /* Takes cdr of a cons pair */

    crate::unroll_parameters!(
        parameters,
        &format!("Liszp: expected syntax ({} <cons pair>)", name)[..],
        true ;
        k, x
    );

    let mut resolved = resolve_value(x, env);

    if let Value::Quote(cons) = &*resolved {
        resolved = Rc::clone(cons);
    }

    let cdr = if let Value::Cons { cdr, .. } = &*resolved {
        Rc::clone(cdr)
    } else {
        panic!("Liszp: function {} expected to receive a cons pair", name);
    };

    return crate::refcount_list![ k, &cdr ];
}

/* Type checking */

pub (in crate::eval) fn is_nil(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Returns whether the arg is a Value::Nil */

    crate::unroll_parameters! {
        parameters,
        "Liszp: expected syntax (nil? <value>)",
        true ;
        k, v
    };

    let resolved = resolve_value(v, env);

    let result = Rc::new(Value::Bool(match *resolved {
        Value::Nil => true,
        _ => false
    }));

    return crate::refcount_list![ k, &result ];
}

pub (in crate::eval) fn is_cons(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Returns whether the arg is a Value::Nil */

    crate::unroll_parameters! {
        parameters,
        "Liszp: expected syntax (cons? <value>)",
        true ;
        k, v
    };

    let resolved = resolve_value(v, env);

    let result = Rc::new(Value::Bool(match *resolved {
        Value::Cons {..} => true,
        _ => false
    }));

    return crate::refcount_list![ k, &result ];
}

pub (in crate::eval) fn is_int(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Returns whether the arg is a Value::Integer */

    crate::unroll_parameters! {
        parameters,
        "Liszp: expected syntax (int? <value>)",
        true ;
        k, v
    };

    let resolved = resolve_value(v, env);

    let result = Rc::new(Value::Bool(match *resolved {
        Value::Integer(_) => true,
        _ => false
    }));

    return crate::refcount_list![ k, &result ];
}

pub (in crate::eval) fn is_float(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Returns whether the arg is a Value::Float */

    crate::unroll_parameters! {
        parameters,
        "Liszp: expected syntax (float? <value>)",
        true ;
        k, v
    };

    let resolved = resolve_value(v, env);

    let result = Rc::new(Value::Bool(match *resolved {
        Value::Float(_) => true,
        _ => false
    }));

    return crate::refcount_list![ k, &result ];
}

pub (in crate::eval) fn is_bool(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Returns whether the arg is a Value::Bool */

    crate::unroll_parameters! {
        parameters,
        "Liszp: expected syntax (bool? <value>)",
        true ;
        k, v
    };

    let resolved = resolve_value(v, env);

    let result = Rc::new(Value::Bool(match *resolved {
        Value::Bool(_) => true,
        _ => false
    }));

    return crate::refcount_list![ k, &result ];
}

pub (in crate::eval) fn is_string(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Returns whether the arg is a Value::String */

    crate::unroll_parameters! {
        parameters,
        "Liszp: expected syntax (str? <value>)",
        true ;
        k, v
    };

    let resolved = resolve_value(v, env);

    let result = Rc::new(Value::Bool(match *resolved {
        Value::String(_) => true,
        _ => false
    }));

    return crate::refcount_list![ k, &result ];
}

pub (in crate::eval) fn is_quote(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Returns whether the arg is a Value::Quote */

    crate::unroll_parameters! {
        parameters,
        "Liszp: expected syntax (quote? <value>)",
        true ;
        k, v
    };

    let resolved = resolve_value(v, env);

    let result = Rc::new(Value::Bool(match *resolved {
        Value::Quote(_) => true,
        _ => false
    }));

    return crate::refcount_list![ k, &result ];
}

pub (in crate::eval) fn is_name(parameters: &Rc<Value>, env: &Env) -> Rc<Value> {
    /* Returns whether the arg is a Value::Name */

    crate::unroll_parameters! {
        parameters,
        "Liszp: expected syntax (name? <value>)",
        true ;
        k, v
    };

    let resolved = resolve_value(v, env);

    let result = Rc::new(Value::Bool(match *resolved {
        Value::Name(_) => true,
        _ => false
    }));

    return crate::refcount_list![ k, &result ];
}
