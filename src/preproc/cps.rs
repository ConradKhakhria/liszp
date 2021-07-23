use crate::read::Value;
use crate::unroll_parameters;
use crate::refcount_list;

use std::collections::LinkedList;
use std::rc::Rc;

type Vals = LinkedList<(Rc<Value>, usize)>;

fn dfs_value_collect(current: &mut Rc<Value>, vals: &mut Vals) {
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
                    car: Rc::new(Value::Name("k@@".into())),
                    cdr: Rc::clone(args)
                });

                let body = if let Value::Cons { .. } = &**body {
                    convert(
                        Rc::clone(body),
                        Some(Rc::new(Value::Name("k@@".into()))),
                        None
                    )
                } else {
                    refcount_list![ Rc::new(Value::Name("k@@".into())), Rc::clone(body) ]
                };

                *current = refcount_list![ lambda_kwd, &args, &body ];
            },

            _ => {
                let mut new_cons_list = Rc::new(Value::Nil);
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
                *current = Rc::new(Value::Name(format!("k{}@@", vals.len())));
            }
        }
    }
}

pub (in crate::preproc) fn convert(value: Rc<Value>, main_continuation: Option<Rc<Value>>, values: Option<Vals>) -> Rc<Value> {
    /* Converts a lambda expression to continuation-passing style */

    let mut root = value.clone();
    let mut vals = values.unwrap_or(LinkedList::new());

    dfs_value_collect(&mut root, &mut vals);

    let mut layered = false;
    let mut converted = main_continuation
                            .unwrap_or(Rc::new(Value::Name("no-continuation".into())));

    for (expr, cont_num) in vals.iter() {
        if let Value::Cons { car, cdr } = &**expr {
            let continuation = if layered {
                refcount_list![
                    Value::Name("lambda&".into()).refcounted(),
                    Value::Name(format!("k{}@@", cont_num)).refcounted(),
                    converted
                ]
            } else {
                layered = true;
                converted
            };

            converted = Rc::new(Value::Cons {
                car: continuation,
                cdr: Rc::clone(cdr)
            });

            converted = Rc::new(Value::Cons {
                car: Rc::clone(car),
                cdr: converted
            });
        }
    }

    return converted;
}
