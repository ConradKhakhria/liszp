use crate::read::Value;
use crate::unroll_parameters;
use crate::refcount_list;


use std::collections::LinkedList;
use std::rc::Rc;


macro_rules! cursor_next {
    ($cursor:expr, $err:expr) => {
        if let Value::Cons { car: val, cdr: rest } = &**$cursor {
            $cursor = rest;
            val
        } else {
            panic!("{}", $err);
        }
    }
}


#[allow(unused_assignments)]
fn find_if(expr: &Rc<Value>) -> Option<(&Value, Rc<Value>, Rc<Value>, Rc<Value>)> {
    /* Attempts to find an if expression and return its constituent parts */

    let if_error = "Liszp: expected syntax (if <cond> <true-case> <false-case>)";

    return if let Value::Cons { car, cdr } = &**expr {
        if (**car).name() == "if&" {
            let mut cursor = cdr;

            let cond  = cursor_next!(cursor, if_error);
            let tcase = cursor_next!(cursor, if_error);
            let fcase = cursor_next!(cursor, if_error);

            Some((
                expr,
                Rc::clone(cond),
                Rc::clone(tcase),
                Rc::clone(fcase)
            ))
        } else if (**car).name() == "lambda&" {
            None
        } else {
            let fcar = find_if(car);
            let fcdr = find_if(cdr);

            if fcar.is_some() {
                fcar
            } else {
                fcdr
            }
        }
    } else {
        None
    }
}


fn replace(expr: &Rc<Value>, old: &Value, new: Rc<Value>) -> Rc<Value> {
    /* Replaces value 'old' with value 'new' in expression */

    return if std::ptr::eq(&**expr, old) {
        new
    } else if let Value::Cons { car, cdr } = &**expr {
        Rc::new(Value::Cons {
            car: replace(car, old, new.clone()),
            cdr: replace(cdr, old, new)
        })
    } else {
        Rc::clone(expr)
    };
}


pub fn move_ifs(expr: Rc<Value>) -> Rc<Value> {
   /* Moves all if expression to the 'effective top level'
    *
    * note
    * ----
    * Lambda sub-expressions are ignored for the following reasons:
    *   1. As the 'let' is implemented with lambda expressions, moving an 'if'
    *      that occured inside a let to outside of it could break bindings.
    *
    *   2. Lambda expressions are converted to CPS with their own
    *      call to convert(), and therefore don't need to be scanned
    *      in the first place.
    */

    if let Some(( if_expr, cond, tcase, fcase )) = find_if(&expr) {
        let tbranch = move_ifs(replace(&expr, if_expr, tcase));
        let fbranch = move_ifs(replace(&expr, if_expr, fcase));

        return refcount_list![
            Value::Name("if&".into()).rc(),
            cond,
            tbranch,
            fbranch
        ];
    } else {
        return expr;
    }
}


fn dfs_value_collect(current: &mut Rc<Value>, vals: &mut LinkedList<(Rc<Value>, usize)>) {
    /* Collects all of the lists depth-first in 'value' into a linked list */

    if let Value::Cons { car: first, .. } = &**current {
        let first_word = &(**first).name()[..];

        match first_word {
            "lambda&" => {
                unroll_parameters! {
                    current,
                    "Liszp: expected syntax (lambda <args> <body>)",
                    false ;
                    lambda_kwd, args, body
                };

                let args = Rc::new(Value::Cons {
                    car: Value::Name("k@@".into()).rc(),
                    cdr: Rc::clone(args)
                });

                let body = if let Value::Cons { .. } = &**body {
                    convert(
                        Rc::clone(body),
                        Some(Value::Name("k@@".into()).rc())
                    )
                } else {
                    refcount_list![ Value::Name("k@@".into()).rc(), Rc::clone(body) ]
                };

                *current = refcount_list![ lambda_kwd, &args, &body ];
            },

            "quote&" => {
                vals.push_front((Rc::clone(current), vals.len() + 1));
                *current = Value::Name(format!("k{}@@", vals.len())).rc();
            },

            _ => {
                let mut new_cons_list = Value::Nil.rc();
                let mut list = (**current).to_list()
                                          .expect("Liszp: internal error in dfs_value_collect() :: 2");
        
                for elem in list.iter_mut() {
                    dfs_value_collect(elem, vals);
                }
        
                for elem in list.iter().rev() {
                    new_cons_list = Rc::new(Value::Cons {
                        car: Rc::clone(elem),
                        cdr: new_cons_list
                    });
                }
        
                vals.push_front((Rc::clone(&new_cons_list), vals.len() + 1));
                *current = Value::Name(format!("k{}@@", vals.len())).rc();
            }
        }
    }
}


#[allow(unused_assignments)]
pub (in crate::preproc) fn convert(value: Rc<Value>, main_continuation: Option<Rc<Value>>) -> Rc<Value> {
    /* Converts a lambda expression to continuation-passing style */

    // Special case for 'if' expressions
    if let Value::Cons { car, cdr } = &*value {
        if (**car).name() == "if&" {
            let if_error = "Liszp: expected syntax (if <cond> <true-case> <false-case>)";

            let mut cursor = cdr;

            let condition = cursor_next!(cursor, if_error);
            let tbranch = cursor_next!(cursor, if_error);
            let fbranch = cursor_next!(cursor, if_error);

            let expr = refcount_list![
                Value::Name("if&".into()).rc(),
                Value::Name("k-if@@".into()).rc(),
                convert(Rc::clone(tbranch), main_continuation.clone()),
                convert(Rc::clone(fbranch), main_continuation)
            ];

            return convert(
                Rc::clone(condition),
                Some(refcount_list![
                    Value::Name("lambda&".into()).rc(),
                    Value::Name("k-if@@".into()).rc(),
                    expr
                ])
            );
        }
    }

    let mut root = value.clone();
    let mut vals = LinkedList::new();

    dfs_value_collect(&mut root, &mut vals);

    let mut layered = false;
    let mut converted_expression = main_continuation
                            .unwrap_or(Value::Name("no-continuation".into()).rc());

    for (expr, cont_num) in vals.iter() {
        if let Value::Cons { car, cdr } = &**expr {
            let continuation = if layered {
                refcount_list![
                    Value::Name("lambda&".into()).rc(),
                    Value::Name(format!("k{}@@", cont_num)).rc(),
                    converted_expression
                ]
            } else {
                layered = true;
                converted_expression
            };

            converted_expression = Rc::new(Value::Cons {
                car: continuation,
                cdr: Rc::clone(cdr)
            });

            converted_expression = Rc::new(Value::Cons {
                car: Rc::clone(car),
                cdr: converted_expression
            });
        }
    }

    // If the result is a tail-level atom 'a' => (k a)
    if !layered {
        converted_expression = refcount_list![
            converted_expression,
            value
        ];
    }

    return converted_expression;
}
