use crate::parse::Value;
use std::collections::LinkedList;

type NameSpace = LinkedList<(Box<String>, Box<Value>)>;

fn resolve_value(value_ref: &Value, local: &NameSpace, global: &NameSpace) -> Box<Value> {
   /* Searches the local and global namespaces for a value name, in case
    * the supplied value is an identifier
    *
    * args
    * ----
    * - value_ref: a pointer to the value to be (potentially) resolved
    * - local: the local value namespace
    * - global: the global value namespace
    *
    * returns
    * -------
    * A value that isn't a value name. If value_ref wasn't an identifier name
    * initially then nothing happens.
    */

    let mut value = value_ref;

    while let Value::Name(name) = value {
        let mut found_var = false;

        for ns in vec![ local, global ].iter() {
            for (n, v) in ns.iter() {
                if *name == **n {
                    value     = v;
                    found_var = true;
                } else if &name[..] == "no-continuation" {
                    return Box::new(Value::Name("no-continuation".into()));
                }
            }
        }

        if !found_var {
            panic!("Unbound value name '{}'", name);
        }
    }

    return Box::new(value.clone());
}

fn bind_function_args<'e>(function: &'e Value, given_args: &'e Value, local: &mut NameSpace) -> Box<Value> {
   /* Binds the function's arguments to their names in the local namespace
    *
    * args
    * ----
    * - function: the lambda expression to be bound.
    * - given_args: the argument values supplied in the function call.
    * - local: the local namespace where the values are to be bound.
    *
    * returns
    * -------
    * The function body to be evaluated.
    *
    * modifies
    * --------
    * The local namespace, by adding the bindings to it. 
    */

    println!("function = {}", function);

    let args = function.index(1);
    let body = function.index(2);

    if !args.is_cons() {
        panic!("Expected list of arguments in lambda expression");
    } else if args.len() != given_args.len() {
        panic!("Function takes {} arguments but was supplied with {}", args.len(), given_args.len());
    }

    for i in 0..args.len() {
        let val = given_args.index(i as usize);
        let name = if let Value::Name(n) = args.index(i as usize) {
            n  
        } else {
            panic!("Expected argument name in lambda expression");
        };
    
        local.push_front((
            Box::new(name.clone()),
            Box::new(val.clone())
        ));
    }

    return Box::new(body.clone());
}

fn valid_lambda(lambda_cdr: &Box<Value>) -> bool {
    /* Checks if the expression is a valid lambda function */

    if let Value::Cons { car: args, cdr: body} = &**lambda_cdr {
        return args.is_cons() && body.is_cons() && body.len() == 1;
    } else {
        return false;
    }
}

fn define_value<'e>(binding: &Value, global: &mut NameSpace) {
    /* Adds a value to the global namespace */

    if !binding.is_cons() {
        panic!("Liszp: Expected def expression with syntax (def <name> <value>)");
    } else if binding.len() != 2 {
        panic!("Liszp: def expression received {} arguments but expected 2", binding.len());
    }

    let value = binding.index(1);
    let name = if let Value::Name(s) = binding.index(0) {
        s
    } else {
        panic!("Liszp: Expected name in def expr");
    };

    global.push_front((
        Box::new(name.clone()),
        Box::new(value.clone())
    ));
}

fn evaluate_if<'e>(body: &'e Value, local: &NameSpace, global: &NameSpace) -> Box<Value> {
    /* Evaluates an if expression */

    if body.len() != 3 {
        panic!("Liszp: if expression received {} arguments but expected length 3", body.len());
    }

    let texpr = body.index(1);
    let fexpr = body.index(2);

    let cond = match *resolve_value(body.index(0), local, global) {
        Value::Bool(b) => b,
        _ => panic!("Liszp: expected boolean value for if statement condition")
    };

    return if cond {
        Box::new(texpr.clone())
    } else {
        Box::new(fexpr.clone())
    };
}

fn evaluate_print<'e>(name: &'e String, rest: &'e Value, local: &NameSpace, global: &NameSpace) -> Box<Value> {
    /* Prints a value and then returns it */

    if !rest.is_cons() {
        panic!("Expected function {} to have arguments", name);
    } else if rest.len() != 2 {
        panic!("Liszp: function {} supplied {} arguments, but expected 1", name, rest.len() - 1);
    }

    let params = rest.to_list();

    let mut p_iter = params.iter();

    let p = resolve_value(p_iter.next().unwrap(), local, global);
    let k = p_iter.next().unwrap();
    

    if &name[..] == "print" {
        print!("{}", p);
    } else {
        println!("{}", p);
    }

    return Box::new(Value::Cons {
        car: k.clone(),
        cdr: Box::new(Value::Cons {
            car: p.clone(),
            cdr: Box::new(Value::Nil)
        })
    });
}

fn eval_integer_arithmetic(op: &String, params: LinkedList<Box<Value>>) -> Box<Value> {
    /* Evaluates an arithmetic function call with an integer value */

    let mut value = if let Value::Integer(i) = *params.front().unwrap().clone() {
        i
    } else {
        panic!("Liszp: expected numeric argument for {} function call", op);
    };

    for x in params.iter().dropping(1) {
        if let Value::Integer(i) = *x.clone() {
            match &op[..] {
                "+&" => value += i,
                "-&" => value -= i,
                "*&" => value *= i,
                "/&" => value /= i,
                 _  => value %= i
            };
        } else {
            panic!("Liszp: expected numeric argument for {} function call", op);
        }
    }

    return Box::new(Value::Integer(if &op[..] == "-" && params.len() == 1 {
        -value
    } else {
        value
    }));
}

fn eval_float_arithmetic(op: &String, params: LinkedList<Box<Value>>) -> Box<Value> {
    /* Evaluates an arithmetic function call with a floating point value */

    let mut value = if let Value::Float(f) = *params.front().unwrap().clone() {
        f
    } else {
        panic!("Liszp: expected numeric argument for {} function call", op);
    };

    for x in params.iter().dropping(1) {
        if let Value::Float(f) = *x.clone() {
            match &op[..] {
                "+&" => value += f,
                "-&" => value -= f,
                "*&" => value *= f,
                "/&" => value /= f,
                 _  => value %= f
            };
        } else {
            panic!("Liszp: expected numeric argument for {} function call", op);
        }
    }

    return Box::new(Value::Float(if &op[..] == "-" && params.len() == 1 {
        -value
    } else {
        value
    }));
}

fn eval_arithmetic(op: &String, all_params: Box<Value>, local: &NameSpace, global: &NameSpace) -> Box<Value> {
   /* Attempts to evaluate an arithmetic expression
    *
    * args
    * ----
    * - op : the string of the possible arithmetic operation.
    * - all_params : the arguments supplied in the function call expression (including
    *                the continuation).
    * - local: the local value namespace.
    * - global: the global value namespace.
    *
    * returns
    * -------
    * If op is an arithmetic expression then Some(the value) else None.
    */

    let mut funcall_parameters = LinkedList::new();
    let continuation;

    let mut is_float = false;

    if all_params.is_cons() {
        let length = all_params.len();
        let plist = all_params.to_list();

        if length == 1 {
            panic!("Received empty {} expression", op);
        }

        for p in plist.iter().take(length as usize - 1) {
            let resolved = resolve_value(&**p, local, global);

            if let Value::Float(_) = *resolved {
                is_float = true;
            }

            funcall_parameters.push_back(resolved);
        }

        continuation = plist.back().unwrap().clone();
    } else {
        panic!("Liszp: expected list of parameters for {} function call", op);
    }

    let value = if is_float {
        eval_float_arithmetic(op, funcall_parameters)
    } else {
        eval_integer_arithmetic(op, funcall_parameters)
    };

    return Box::new(Value::Cons {
        car: continuation,
        cdr: Box::new(Value::Cons {
            car: value,
            cdr: Box::new(Value::Nil)
        })
    });
}

fn eval_comparison(op: &String, all_params: Box<Value>, local: &NameSpace, global: &NameSpace) -> Box<Value> {
   /* Attempts to evaluate a comparison expression
    *
    * args
    * ----
    * - op : the comparison operator
    * - all_params : all (both) parameters supplied to the function (as well as the continuation)
    * - local : the local namespace.
    * - global : the global namespace.
    *
    * returns
    * -------
    * The boolean value resulting from the comparison.
    */

    let op_len = op.len();

    if all_params.len() != 3 {
        panic!("{} expression expected 2 arguments but received {}",&op[..op_len-1], all_params.len() - 1);
    }

    let params = all_params.to_list();
    let mut params_iter = params.iter();

    let a = resolve_value(params_iter.next().unwrap(), local, global);
    let b = resolve_value(params_iter.next().unwrap(), local, global);
    let k = params_iter.next().unwrap();

    let result = match (*a.clone(), *b.clone()) {
        (Value::Integer(x), Value::Integer(y)) => {
            match &op[..] {
                "<&"  => x < y,
                ">&"  => x > y,
                "<=&" => x <= y,
                ">=&" => x >= y,
                "==&" => x == y,
                "!=&" => x != y,
                _     => panic!("{} not a comparison operator", op)
            }
        },

        (Value::Integer(x), Value::Float(y)) => {
            match &op[..] {
                "<&"  => x < y,
                ">&"  => x > y,
                "<=&" => x <= y,
                ">=&" => x >= y,
                "==&" => x == y,
                "!=&" => x != y,
                _     => panic!("{} not a comparison operator", op)
            }
        },

        (Value::Float(x), Value::Integer(y)) => {
            match &op[..] {
                "<&"  => x < y,
                ">&"  => x > y,
                "<=&" => x <= y,
                ">=&" => x >= y,
                "==&" => x == y,
                "!=&" => x != y,
                _     => panic!("{} not a comparison operator", op)
            }
        },

        (Value::Float(x), Value::Float(y)) => {
            match &op[..] {
                "<&"  => x < y,
                ">&"  => x > y,
                "<=&" => x <= y,
                ">=&" => x >= y,
                "==&" => x == y,
                "!=&" => x != y,
                _     => panic!("{} not a comparison operator", op)
            }
        },

        _ => panic!("Expected 2 numeric values in {} expression", &op[..op_len-2])
    };

    return Box::new(Value::Cons {
        car: k.clone(),
        cdr: Box::new(Value::Cons {
            car: Box::new(Value::Bool(result)),
            cdr: Box::new(Value::Nil)
        })
    });
}

pub fn eval(exprs: LinkedList<Value>) -> LinkedList<Value> {
   /* Evaluates a list of expressions
    *
    * args
    * ----
    * - exprs: a linked list of all the expressions to evaluate
    *
    * returns
    * -------
    * The list with each expression evaluated
    *
    * note
    * ----
    * As this function both takes in and returns a list of expressions, this
    * function essentially just reduces each element of exprs to an atomic
    * expression.
    */

    let mut evaluated = LinkedList::new();
    let mut global = LinkedList::new();
    let mut local = LinkedList::new();

    for expr in exprs.iter() {
        let mut value = Box::new(expr.clone());

        // Trampoline
        while let Value::Cons { car: first, cdr: rest } = *value {
            let function = match *first.clone() {
                // Attempt to resolve the function name
                Value::Name(function_name) => {
                    match &function_name[..] {
                        "lambda&" => {
                            if valid_lambda(&rest) {
                                value = Box::new(Value::Lambda {
                                    args: Box::new(rest.index(0).clone()),
                                    body: Box::new(rest.index(1).clone())
                                });

                                continue;
                            } else {
                                panic!("Liszp: invalid lambda syntax");
                            }
                        },

                        "def&" => {
                            define_value(&rest, &mut global);
                            value = Box::new(Value::Nil);
                            continue;
                        },

                        "if&" => {
                            value = Box::new(*evaluate_if(&rest, &local, &global));
                            continue;
                        },

                        "print&"|"println&" => {
                            value = evaluate_print(&function_name, &*rest, &local, &global);
                            continue;
                        },

                        "quote&"|"\"&" => {
                            value = Box::new(Value::Quote(rest));
                            continue;
                        },

                        "car&"|"first&" => {
                            if rest.len() != 2 {
                                panic!("Received {} arguments in 'car' expr, expected 1", rest.len() - 1);
                            } else if let Value::Cons { car: cons_value, cdr: cont } = *rest {
                                let continuation = cont.index(0);
                                let car_value = if let Value::Cons { car, .. } = *cons_value {
                                    car
                                } else {
                                    panic!("Cannot evaluate car of non-cons expression");
                                };

                                value = Box::new(Value::Cons {
                                    car: Box::new(continuation.clone()),
                                    cdr: Box::new(Value::Cons {
                                        car: car_value,
                                        cdr: Box::new(Value::Nil)
                                    })
                                });

                                continue;
                            } else {
                                panic!("Cannot evaluate car of non-cons expression");
                            }
                        },

                        "cdr&"|"rest&" => {
                            if rest.len() != 2 {
                                panic!("Received {} arguments in 'car' expr, expected 1", rest.len() - 1);
                            } else if let Value::Cons { car: cons_value, cdr: cont } = *rest {
                                let continuation = cont.index(0);
                                let cdr_value = if cons_value.len() == 2 {
                                    Box::new(cons_value.index(1).clone())
                                } else {
                                    panic!("Cannot evaluate car of non-cons expression");
                                };

                                value = Box::new(Value::Cons {
                                    car: Box::new(continuation.clone()),
                                    cdr: Box::new(Value::Cons {
                                        car: cdr_value,
                                        cdr: Box::new(Value::Nil)
                                    })
                                });

                                continue;
                            } else {
                                panic!("Cannot evaluate car of non-cons expression");
                            }
                        },

                        "cons&"|"join&" => {
                            let values = if rest.len() == 3 {
                                rest.to_list()
                            } else {
                                panic!("cons expr received {} arguments but expected 2", rest.len() - 2);
                            };

                            let mut v_iter = values.iter();

                            let x = v_iter.next().unwrap();
                            let y = v_iter.next().unwrap();
                            let k = v_iter.next().unwrap();

                            // Create the cons
                            value = Box::new(Value::Cons {
                                car: x.clone(),
                                cdr: y.clone()
                            });

                            // Turn it into a thunk
                            value = Box::new(Value::Cons {
                                car: k.clone(),
                                cdr: Box::new(Value::Cons {
                                    car: value,
                                    cdr: Box::new(Value::Nil)
                                })
                            });

                            continue;
                        },

                        // Type checking
                        "int?&" => {
                            let vals = if rest.len() == 2 {
                                rest.to_list()
                            } else {
                                panic!("int? expression expected 1 argument but received {}", rest.len() - 1);
                            };

                            let mut v_iter = vals.iter();

                            let x = v_iter.next().unwrap();
                            let k = v_iter.next().unwrap();

                            value = Box::new(Value::Cons {
                                car: k.clone(),
                                cdr: Box::new(Value::Cons {
                                    car: Box::new(if let Value::Integer(_) = **x {
                                        Value::Bool(true)
                                    } else {
                                        Value::Bool(false)
                                    }),
                                    cdr: Box::new(Value::Nil)
                                })
                            });

                            continue;
                        },

                        "float?&" => {
                            let vals = if rest.len() == 2 {
                                rest.to_list()
                            } else {
                                panic!("float? expression expected 1 argument but received {}", rest.len() - 1);
                            };

                            let mut v_iter = vals.iter();

                            let x = v_iter.next().unwrap();
                            let k = v_iter.next().unwrap();

                            value = Box::new(Value::Cons {
                                car: k.clone(),
                                cdr: Box::new(Value::Cons {
                                    car: Box::new(if let Value::Float(_) = **x {
                                        Value::Bool(true)
                                    } else {
                                        Value::Bool(false)
                                    }),
                                    cdr: Box::new(Value::Nil)
                                })
                            });

                            continue;
                        },

                        "str?&" => {
                            let vals = if rest.len() == 2 {
                                rest.to_list()
                            } else {
                                panic!("str? expression expected 1 argument but received {}", rest.len() - 1);
                            };

                            let mut v_iter = vals.iter();

                            let x = v_iter.next().unwrap();
                            let k = v_iter.next().unwrap();

                            value = Box::new(Value::Cons {
                                car: k.clone(),
                                cdr: Box::new(Value::Cons {
                                    car: Box::new(if let Value::String(_) = **x {
                                        Value::Bool(true)
                                    } else {
                                        Value::Bool(false)
                                    }),
                                    cdr: Box::new(Value::Nil)
                                })
                            });

                            continue;
                        },

                        "bool?&" => {
                            let vals = if rest.len() == 2 {
                                rest.to_list()
                            } else {
                                panic!("bool? expression expected 1 argument but received {}", rest.len() - 1);
                            };

                            let mut v_iter = vals.iter();

                            let x = v_iter.next().unwrap();
                            let k = v_iter.next().unwrap();

                            value = Box::new(Value::Cons {
                                car: k.clone(),
                                cdr: Box::new(Value::Cons {
                                    car: Box::new(if let Value::Bool(_) = **x {
                                        Value::Bool(true)
                                    } else {
                                        Value::Bool(false)
                                    }),
                                    cdr: Box::new(Value::Nil)
                                })
                            });

                            continue;
                        },

                        "cons?&"|"pair?&" => {
                            let vals = if rest.len() == 2 {
                                rest.to_list()
                            } else {
                                panic!("cons? expression expected 1 argument but received {}", rest.len() - 1);
                            };

                            let mut v_iter = vals.iter();

                            let x = v_iter.next().unwrap();
                            let k = v_iter.next().unwrap();

                            value = Box::new(Value::Cons {
                                car: k.clone(),
                                cdr: Box::new(Value::Cons {
                                    car: Box::new(if let Value::Cons {..} = **x {
                                        Value::Bool(true)
                                    } else {
                                        Value::Bool(false)
                                    }),
                                    cdr: Box::new(Value::Nil)
                                })
                            });

                            continue;
                        },

                        "lambda?&" => {
                            let vals = if rest.len() == 2 {
                                rest.to_list()
                            } else {
                                panic!("lambda? expression expected 1 argument but received {}", rest.len() - 1);
                            };

                            let mut v_iter = vals.iter();

                            let x = v_iter.next().unwrap();
                            let k = v_iter.next().unwrap();

                            value = Box::new(Value::Cons {
                                car: k.clone(),
                                cdr: Box::new(Value::Cons {
                                    car: Box::new(if let Value::Lambda {..} = **x {
                                        Value::Bool(true)
                                    } else {
                                        Value::Bool(false)
                                    }),
                                    cdr: Box::new(Value::Nil)
                                })
                            });

                            continue;
                        },

                        "quote?&" => {
                            let vals = if rest.len() == 2 {
                                rest.to_list()
                            } else {
                                panic!("quote? expression expected 1 argument but received {}", rest.len() - 1);
                            };

                            let mut v_iter = vals.iter();

                            let x = v_iter.next().unwrap();
                            let k = v_iter.next().unwrap();

                            value = Box::new(Value::Cons {
                                car: k.clone(),
                                cdr: Box::new(Value::Cons {
                                    car: Box::new(if let Value::Quote {..} = **x {
                                        Value::Bool(true)
                                    } else {
                                        Value::Bool(false)
                                    }),
                                    cdr: Box::new(Value::Nil)
                                })
                            });

                            continue;
                        },

                        "nil?&" => {
                            let vals = if rest.len() == 2 {
                                rest.to_list()
                            } else {
                                panic!("nil? expression expected 1 argument but received {}", rest.len() - 1);
                            };

                            let mut v_iter = vals.iter();

                            let x = v_iter.next().unwrap();
                            let k = v_iter.next().unwrap();

                            value = Box::new(Value::Cons {
                                car: k.clone(),
                                cdr: Box::new(Value::Cons {
                                    car: Box::new(if let Value::Nil = **x {
                                        Value::Bool(true)
                                    } else {
                                        Value::Bool(false)
                                    }),
                                    cdr: Box::new(Value::Nil)
                                })
                            });

                            continue;
                        },

                        "len&" => {
                            let vals = if rest.len() == 2 {
                                rest.to_list()
                            } else {
                                panic!("nil? expression expected 1 argument but received {}", rest.len() - 1);
                            };

                            let mut v_iter = vals.iter();

                            let x = resolve_value(v_iter.next().unwrap(), &local, &global);
                            let k = v_iter.next().unwrap();

                            value = Box::new(Value::Cons {
                                car: k.clone(),
                                cdr: Box::new(Value::Cons {
                                    car: Box::new(Value::Integer(rug::Integer::from(x.len()))),
                                    cdr: Box::new(Value::Nil)
                                })
                            });

                            continue;
                        }

                        "no-continuation" => {
                            if rest.len() == 1 {
                                value = Box::new(rest.index(0).clone());
                                break;
                            } else {
                                panic!("Liszp : unexpected internal error in eval() :: 1");
                            }
                        },

                        "+&"|"-&"|"*&"|"/&"|"%&" => {
                            value = eval_arithmetic(&function_name, rest, &local, &global);
                            continue;
                        },

                        "<&"|">&"|"<=&"|">=&"|"==&"|"!=&" => {
                            value = eval_comparison(&function_name, rest, &local, &global);
                            continue;
                        },

                        _ => resolve_value(&*first, &local, &global)
                    }
                },

                Value::Cons { .. } => first,

                _ => panic!("Expected function name or literal at start of expression")
            };

            value = Box::new(*bind_function_args(&*function, &*rest, &mut local));
        }

        evaluated.push_back(*value.clone());
    }

    return evaluated;
}
