use crate::{
    read::Value,
    refcount_list
};
use std::{
    collections::HashMap,
    rc::Rc
};
use itertools::Itertools;
use rug;


type ValueMap = HashMap<String, Rc<Value>>;

pub struct Env {
    globals: ValueMap
}


impl Env {
    pub fn new() -> Self {
        Env {
            globals: HashMap::new()
        }
    }


    /* Getters */

    pub fn get_globals(&self) -> &ValueMap {
        &self.globals
    }


    /* Env-related functions */

    fn resolve(&self, value: &Rc<Value>) -> Rc<Value> {
        /* If 'value' is a name, this substitutes it for the ident's value */

        if let Value::Name(name) = &**value {
            self.globals.get(name).expect(format!("Unbound name '{}'", &name[1..]).as_str()).clone()
        } else {
            value.clone()
        }
    }


    /* Eval */

    pub fn eval(&mut self, expr: &Rc<Value>) -> Rc<Value> {
        /* Evaluates an expression in Env */

        let mut value = expr.clone();

        while let Value::Cons { car: function, cdr: args  } = &*value {
            let function_name = function.name();
            let args = args.to_list().expect("Liszp: expected a list of arguments");

            value = match function_name.as_str() {
                "&bool?"            => self.value_is_bool(&args),
                "&car"              => self.car(&args),
                "&cdr"              => self.cdr(&args),
                "&cons"             => self.cons(&args),
                "&cons?"            => self.value_is_cons(&args),
                "&def"              => self.define_value(&args),
                "&equals?"          => self.values_are_equal(&args),
                "&eval"             => self.eval_quoted(&args),
                "&float"            => self.value_is_float(&args),
                "&if"               => self.if_expr(&args),
                "&int?"             => self.value_is_int(&args),
                "&name?"            => self.value_is_name(&args),
                "&nil?"             => self.value_is_nil(&args),
                "no-continuation"   => self.no_continuation(&args),
                "&panic"            => self.panic(&args),
                "&print"            => self.print_value(&args, false),
                "&println"          => self.print_value(&args, true),
                "&quote"            => self.quote_value(&args),
                "&quote?"           => self.value_is_quote(&args),
                "&str?"             => self.value_is_str(&args),
                "&+"|"&-"|"&*"|"&/" => self.arithmetic_expression(&function_name, &args),
                "&%"                => self.modulo(&args),
                "&and"|"&or"|"&xor" => self.binary_logical_operation(&function_name, &args),
                "&not"              => self.logical_negation(&args),
                "&<"|"&>"|"&<="|
                "&>="|"&=="|"&!="   => self.comparison(&function_name, &args),
                _                   => self.evaluate_lambda_funcall(function, &args)
            }
        }

        value
    }


    /* Non-built-in function evaluation */

    fn evaluate_lambda_funcall(&self, function: &Rc<Value>, arg_values: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Evaluates the calling of a non-built-in function */

        let function_components = self.resolve(function)
                                                   .to_list()
                                                   .expect("Liszp: function should have syntax (lambda <args> <body>)");

        if function_components.len() != 3 {
            panic!("Liszp: function should have syntax (lambda <args> <body>)");
        } else if function_components[0].name() != "&lambda" {
            panic!("Liszp: attempt to call a non-function value");
        }

        let arg_names = Self::get_arg_names(&function_components[1]);
        let mut arg_map = Self::build_argument_hashmap(&arg_names, arg_values);

        let function_body = &function_components[2];

        self.recursively_bind_args(function_body, &mut arg_map)
    }


    fn get_arg_names(arg_component: &Rc<Value>) -> Vec<String> {
        /* Gets the list of argument names from the argument component */

        match &**arg_component {
            Value::Cons {..} => {
                let values_list = arg_component.to_list().unwrap();
                let mut names = Vec::with_capacity(values_list.len());

                for v in values_list.iter() {
                    match &**v {
                        Value::Name(name) => names.push(name.clone()),
                        _ => panic!("Liszp: Expected name in function argument")
                    }
                }

                names
            }

            Value::Name(name) => {
                vec![ name.clone() ]
            }

            Value::Nil => vec![],

            _ => panic!("Liszp: Function expected a list of arguments or a single argument in lambda expression")
        }
    }


    fn build_argument_hashmap(arg_names: &Vec<String>, arg_values: &Vec<Rc<Value>>) -> ValueMap {
        /* Builds a map from argument names to argument values */

        let mut hashmap = HashMap::new();

        if arg_names.len() != arg_values.len() {
            panic!("Function takes {} arguments but received {}", arg_names.len(), arg_values.len());
        }

        for i in 0..arg_names.len() {
            hashmap.insert(arg_names[i].clone(), arg_values[i].clone());
        }

        hashmap
    }


    fn recursively_bind_args(&self, expr: &Rc<Value>, arg_map: &mut ValueMap) -> Rc<Value> {
        /* Returns function_body but with argument names replaced with their values */

        match &**expr {
            Value::Name(name) => {
                if let Some(value) = arg_map.get(name) {
                    value.clone()
                } else {
                    expr.clone()
                }
            },

            Value::Cons { car, cdr } => {
                if car.name() == "&lambda" {
                    let lambda_components = expr.to_list().expect("Liszp: malformed lambda expression");
                    let arg_component = &lambda_components[1];
                    let body_component = &lambda_components[2];

                    let shadowed_arguments = Self::remove_shadowed_arguments(arg_component, arg_map);

                    let body_with_bound_arguments = self.recursively_bind_args(body_component, arg_map);

                    arg_map.extend(shadowed_arguments);

                    refcount_list![
                        lambda_components[0].clone(),
                        arg_component.clone(),
                        body_with_bound_arguments
                    ]
                } else {
                    Rc::new(Value::Cons {
                        car: self.recursively_bind_args(car, arg_map),
                        cdr: self.recursively_bind_args(cdr, arg_map)
                    })
                }
            }

            _ => expr.clone()
        }
    }


    fn remove_shadowed_arguments(arg_component: &Rc<Value>, arg_map: &mut ValueMap) -> ValueMap {
        /* Removes any arguments from arg_map that are shadowed in lambda_components */

        let mut shadowed_args = HashMap::new();

        for arg_name in Self::get_arg_names(arg_component) {
            if let Some(removed_value) = arg_map.remove(&arg_name) {
                shadowed_args.insert(arg_name, removed_value);
            }
        }

        shadowed_args
    }


    /* built-in functions */

    fn car(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Gets the car of a cons pair */

        match args.as_slice() {
            [continuation, xs] => {
                let resolved = self.resolve(xs);

                let xs = match &*resolved {
                    Value::Quote(cons) => cons.clone(),
                    _ => unreachable!()
                };

                let quoted_car = match &*xs {
                    Value::Cons { car, .. } => Value::Quote(car.clone()).rc(),
                    _ => panic!("Liszp: function 'cons' expected to receive cons pair")
                };

                refcount_list![ continuation, &quoted_car ]
            }

            _ => panic!("Liszp: function 'car' takes 1 argument")
        }
    }


    fn cdr(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Gets the cdr of a cons pair */

        match args.as_slice() {
            [continuation, xs] => {
                let resolved = self.resolve(xs);

                let xs = match &*resolved {
                    Value::Quote(cons) => cons.clone(),
                    _ => unreachable!()
                };

                let quoted_cdr = match &*xs {
                    Value::Cons { cdr, .. } => Value::Quote(cdr.clone()).rc(),
                    _ => panic!("Liszp: function 'cons' expected to receive cons pair")
                };

                refcount_list![ continuation, &quoted_cdr ]
            },

            _ => panic!("Liszp: function 'cdr' takes 1 argument")
        }
    }


    fn cons(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Creates a cons pair */

        match args.as_slice() {
            [continuation, car, cdr] => {
                let car = self.resolve(car);
                let cdr = self.resolve(cdr);

                let cons_pair = Value::Quote(
                    Rc::new(Value::Cons {
                        car,
                        cdr: if let Value::Quote(v) = &*cdr {
                            v.clone()
                        } else {
                            cdr
                        }
                    })
                );

                refcount_list![ continuation.clone(), cons_pair.rc() ]
            }

            _ => panic!("Liszp: function 'cons' expected 2 arguments")
        }
    }


    fn define_value(&mut self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Defines a value in self.globals */

        if args.len() != 3 {
            panic!("Liszp: expected syntax (def <name> <value>)");
        }

        let continuation = &args[0];
        let name = &args[1];
        let value = &args[2];

        if let Value::Name(name) = &**name {
            self.globals.insert(name.clone(), value.clone());
        } else {
            panic!("Liszp: expected name in def expression");
        }

        refcount_list![ continuation.clone(), Value::Nil.rc() ]
    }


    fn eval_quoted(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Evaluates a quoted value */

        match args.as_slice() {
            [continuation, quoted_value] => {
                let value = if let Value::Quote(v) = &*self.resolve(quoted_value) {
                    v.clone()
                } else {
                    quoted_value.clone()
                };

                refcount_list![ continuation, &value ]
            }

            _ => panic!("Liszp: function 'quote' takes exactly one argument")
        }
    }


    fn if_expr(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Evaluates an if expression */
    
        if args.len() != 3 {
            panic!("Liszp: if expression has syntax (if <condition> <true case> <false case>)");
        }

        let cond = self.resolve(&args[0]);
        let true_case = self.resolve(&args[1]);
        let false_case = self.resolve(&args[2]);

        if let Value::Bool(b) = &*cond {
            if *b {
                true_case
            } else {
                false_case
            }
        } else {
            panic!("if expression expected a boolean condition")
        }
    }


    fn no_continuation(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* The final stage of a trampolined evaluation */

        if args.len() == 1 {
            self.resolve(&args[0])
        } else {
            unreachable!()
        }
    }


    fn panic(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Panics */

        match args.as_slice() {
            [_, msg] => panic!("{}", msg),
            _ => panic!("Liszp: expected syntax (panic <message>)")
        }
    }


    fn print_value(&self, args: &Vec<Rc<Value>>, newline: bool) -> Rc<Value> {
        /* Prints a value, optionally with a newline */

        if args.len() != 2 {
            panic!("Function print{} takes 1 argument only", if newline { "ln" } else { "" });
        }

        let continuation = &args[0];
        let value = self.resolve(&args[1]);

        if newline {
            println!("{}", value);
        } else {
            print!("{}", value);
        }

        refcount_list![ continuation.clone(), value]
    }


    fn quote_value(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Quotes a value */

        match args.as_slice() {
            [continuation, value] => {
                let quoted_value = match &**value {
                    Value::Quote(_) => value.clone(),
                    _ => Value::Quote(self.resolve(value)).rc()
                };

                refcount_list![ continuation, &quoted_value ]
            }

            _ => panic!("Liszp: function 'quote' takes exactly one value")
        }
    }


    fn values_are_equal(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Compares two values */

        match args.as_slice() {
            [continuation, x, y] => {
                let result = Value::Bool(self.resolve(x) == self.resolve(y)).rc();

                refcount_list![ continuation, &result ]
            },

            _ => panic!("Liszp: Function 'equals?' takes exactly 2 parameters")
        }
    }


    fn value_is_bool(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Returns whether a value is a bool */

        match args.as_slice() {
            [continuation, value] => {
                let resolved = self.resolve(value);

                let value = match &*resolved {
                    Value::Quote(v) => v,
                    _ => &resolved
                };

                let result = match &**value {
                    Value::Bool(_) => true,
                    _ => false
                };

                refcount_list![ continuation.clone(), Value::Bool(result).rc() ]
            },

            _ => panic!("Liszp: function 'bool?' takes exactly one argument")
        }
    }


    fn value_is_cons(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Returns whether a value is a cons pair */

        match args.as_slice() {
            [continuation, value] => {
                let resolved = self.resolve(value);

                let value = match &*resolved {
                    Value::Quote(v) => v,
                    _ => &resolved
                };

                let result = match &**value {
                    Value::Cons {..} => true,
                    _ => false
                };

                refcount_list![ continuation.clone(), Value::Bool(result).rc() ]
            },

            _ => panic!("Liszp: function 'cons?' takes exactly one argument")
        }
    }


    fn value_is_float(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Returns whether a value is a float */

        match args.as_slice() {
            [continuation, value] => {
                let resolved = self.resolve(value);

                let value = match &*resolved {
                    Value::Quote(v) => v,
                    _ => &resolved
                };

                let result = match &**value {
                    Value::Float(_) => true,
                    _ => false
                };

                refcount_list![ continuation.clone(), Value::Bool(result).rc() ]
            },

            _ => panic!("Liszp: function 'float?' takes exactly one argument")
        }
    }


    fn value_is_int(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Returns whether a value is an int */

        match args.as_slice() {
            [continuation, value] => {
                let resolved = self.resolve(value);

                let value = match &*resolved {
                    Value::Quote(v) => v,
                    _ => &resolved
                };

                let result = match &**value {
                    Value::Integer(_) => true,
                    _ => false
                };

                refcount_list![ continuation.clone(), Value::Bool(result).rc() ]
            },

            _ => panic!("Liszp: function 'int?' takes exactly one argument")
        }
    }


    fn value_is_nil(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Returns whether a value is nil */

        match args.as_slice() {
            [continuation, value] => {
                let resolved = self.resolve(value);

                let value = match &*resolved {
                    Value::Quote(v) => v,
                    _ => &resolved
                };

                let result = match &**value {
                    Value::Nil => true,
                    _ => false
                };

                refcount_list![ continuation.clone(), Value::Bool(result).rc() ]
            },

            _ => panic!("Liszp: function 'nil?' takes exactly one argument")
        }
    }


    fn value_is_name(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Returns whether a value is a name */

        match args.as_slice() {
            [continuation, value] => {
                let resolved = self.resolve(value);

                let value = match &*resolved {
                    Value::Quote(v) => v,
                    _ => &resolved
                };

                let result = match &**value {
                    Value::Name(_) => true,
                    _ => false
                };

                refcount_list![ continuation.clone(), Value::Bool(result).rc() ]
            },

            _ => panic!("Liszp: function 'name?' takes exactly one argument")
        }
    }


    fn value_is_quote(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Returns whether a value is quoted */

        match args.as_slice() {
            [continuation, value] => {
                let result = match &*self.resolve(value) {
                    Value::Quote(_) => true,
                    _ => false
                };

                refcount_list![ continuation.clone(), Value::Bool(result).rc() ]
            },

            _ => panic!("Liszp: function 'quote?' takes exactly one argument")
        }
    }


    fn value_is_str(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Returns whether a value is a str */

        match args.as_slice() {
            [continuation, value] => {
                let resolved = self.resolve(value);

                let value = match &*resolved {
                    Value::Quote(v) => v,
                    _ => &resolved
                };

                let result = match &**value {
                    Value::String(_) => true,
                    _ => false
                };

                refcount_list![ continuation.clone(), Value::Bool(result).rc() ]
            },

            _ => panic!("Liszp: function 'str?' takes exactly one argument")
        }
    }


    /* Arithmetic */

    fn arithmetic_expression(&self, op: &String, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Computes an arithmetic expression */

        if args.len() < 2 {
            panic!("Liszp: '{}' expression takes at least 1 argument", op);
        }

        let mut numbers = Vec::with_capacity(args.len());
        let continuation = &args[0];
        let mut result_is_float = false;

        for arg in args.iter().dropping(1) {
            let arg = self.resolve(arg);

            match &*arg {
                Value::Float(_) => {
                    result_is_float = true;
                    numbers.push(arg);
                },

                Value::Integer(_) => numbers.push(arg),

                _ => panic!("Liszp: '{}' expression takes numeric arguments", &op[1..])
            }
        }

        let result = if result_is_float {
            Self::float_arithmetic(op, &numbers)
        } else {
            Self::integer_arithmetic(op, &numbers)
        };

        refcount_list![ continuation.clone(), result ]
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


    fn modulo(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Takes the modulus of a number */

        match args.as_slice() {
            [continuation, dividend, divisor] => {
                let dividend = self.resolve(dividend);
                let divisor = self.resolve(divisor);

                let result = match (&*dividend, &*divisor) {
                    (Value::Float(x), Value::Float(y)) => Value::Float(x.clone() % y.clone()).rc(),

                    (Value::Float(_), Value::Integer(_)) => panic!("Liszp: Cannot take the integer modulo of a float"),

                    (Value::Integer(x), Value::Integer(y)) => Value::Integer(x.clone() % y.clone()).rc(),

                    _ => unreachable!()
                };

                refcount_list![ continuation, &result ]
            },

            _ => panic!("Liszp: modulo expressions take exactly 2 arguments")
        }
    }


    /* Logic */

    fn binary_logical_operation(&self, op: &String, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Evaluates a binary logical operation */

        match args.as_slice() {
            [continuation, x, y] => {
                let x = match &*self.resolve(x) {
                    Value::Bool(b) => *b,
                    _ => panic!("Liszp: {} expressions take boolean arguments", &op[1..])
                };

                let y = match &*self.resolve(y) {
                    Value::Bool(b) => *b,
                    _ => panic!("Liszp: {} expressions take boolean arguments", &op[1..])
                };

                let result = match op.as_str() {
                    "&and" => x && y,
                    "&or"  => x || y,
                    "&xor" => x ^ y,
                    _      => unreachable!()
                };

                refcount_list![ continuation.clone(), Value::Bool(result).rc() ]
            }

            _ => panic!("Liszp: {} expressions take exactly 2 arguments", &op[1..])
        }
    }


    fn logical_negation(&self, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Performs a logical not operation */

        match args.as_slice() {
            [continuation, x] => {
                let x = match &*self.resolve(x) {
                    Value::Bool(b) => *b,
                    _ => panic!("Liszp: not expressions take a boolean argument")
                };

                let result = Value::Bool(!x).rc();

                refcount_list![ continuation, &result ]
            }

            _ => panic!("Liszp: not expressions take exactly 1 argument")
        }
    }


    /* Comparison */

    fn comparison(&self, op: &String, args: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Compares two values */

        match args.as_slice() {
            [continuation, x, y] => {
                let x = self.resolve(x);
                let y = self.resolve(y);

                let result = match (&*x, &*y) {
                    (Value::Integer(x), Value::Integer(y)) => {
                        Self::integer_comparison(op, x, y)
                    }

                    (Value::Float(x), Value::Integer(y)) => {
                        let y = rug::Float::with_val(53, y);

                        Self::float_comparison(op, x, &y)
                    }

                    (Value::Integer(x), Value::Float(y)) => {
                        let x = rug::Float::with_val(53, x);

                        Self::float_comparison(op, &x, y)
                    }

                    (Value::Float(x), Value::Float(y)) => {
                        Self::float_comparison(op, x, y)
                    }

                    _ => panic!("Liszp: {} expressions take two numeric values", &op[1..])
                };

                refcount_list![ continuation, &result ]
            }

            _ => panic!("Liszp: {} expressions take exactly 2 values", &op[1..])
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
}
