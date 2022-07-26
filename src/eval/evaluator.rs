use crate::read;
use crate::error::Error;
use crate::new_error;
use crate::preprocess::cps::CPSConverter;
use crate::preprocess::macros::Macro;
use crate::preprocess::preprocess;
use crate::refcount_list;
use crate::value::Value;
use itertools::Itertools;
use rug;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;


type ValueMap = HashMap<String, Rc<Value>>;

pub struct Evaluator {
    evaluated: Vec<Rc<Value>>,
    globals: ValueMap,
    pub macros: HashMap<String, Macro>,
}


impl Evaluator {
    pub fn new() -> Self {
        Evaluator {
            evaluated: vec![],
            globals: HashMap::new(),
            macros: HashMap::new(),
        }
    }


    /* Getters */

    #[allow(dead_code)]
    pub fn get_globals(&self) -> &ValueMap {
        &self.globals
    }


    /* Env-related functions */

    fn resolve(&self, value: &Rc<Value>) -> Result<Rc<Value>, Error> {
        /* If 'value' is a name, this substitutes it for the ident's value */

        if let Value::Name(name) = &**value {
            match self.globals.get(name) {
                Some(v) => Ok(v.clone()),
                None => new_error!("unbound name '{}'", &name[1..]).into()
            }
        } else {
            Ok(value.clone())
        }
    }


    /* Eval */

    pub fn eval(&mut self, expr: &Rc<Value>) -> Result<Rc<Value>, Error> {
        /* Evaluates an expression in Env */

        let mut value = match preprocess(expr, self)? {
            Some(v) => v,
            None => return Ok(Value::Nil.rc())
        };

        while let Value::Cons { car: function, cdr: args  } = &*value {
            let function_name = function.name();
            let args = args.to_list().expect("Liszp: expected a list of arguments");

            value = match function_name.as_str() {
                "&bool?"            => self.value_is_bool(&args)?,
                "&car"              => self.car(&args)?,
                "&cdr"              => self.cdr(&args)?,
                "&cons"             => self.cons(&args)?,
                "&cons?"            => self.value_is_cons(&args)?,
                "&def"              => self.define_value(&args)?,
                "&equals?"          => self.values_are_equal(&args)?,
                "&eval"             => self.eval_quoted(&args)?,
                "&float"            => self.value_is_float(&args)?,
                "&if"               => self.if_expr(&args)?,
                "&int?"             => self.value_is_int(&args)?,
                "&name?"            => self.value_is_name(&args)?,
                "&nil?"             => self.value_is_nil(&args)?,
                "no-continuation"   => self.no_continuation(&args)?,
                "&panic"            => self.panic(&args)?,
                "&print"            => self.print_value(&args, false)?,
                "&println"          => self.print_value(&args, true)?,
                "&quote"            => self.quote_value(&args)?,
                "&quote?"           => self.value_is_quote(&args)?,
                "&str?"             => self.value_is_str(&args)?,
                "&+"|"&-"|"&*"|"&/" => self.arithmetic_expression(&function_name, &args)?,
                "&%"                => self.modulo(&args)?,
                "&and"|"&or"|"&xor" => self.binary_logical_operation(&function_name, &args)?,
                "&not"              => self.logical_negation(&args)?,
                "&<"|"&>"|"&<="|
                "&>="|"&=="|"&!="   => self.comparison(&function_name, &args)?,
                _                   => self.evaluate_lambda_funcall(function, &args)?,
            }
        }

        Ok(value)
    }


    pub fn eval_file<P: AsRef<Path> + ToString>(&mut self, filepath: P) -> Result<(), Error> {
       /* Evaluates a source file */

        let filename = filepath.to_string();

        let source = std::fs::read_to_string(filepath)
                        .expect(format!("Cannot open file '{}'", filename).as_str());

        for expr in read::read(&source, &filename)?.iter() {
            let evaluated = self.eval(expr)?;

            self.evaluated.push(evaluated);
        }

        Ok(())
    }


    pub fn eval_source_string<S: Into<String>>(&mut self, source: &String, filename: S) -> Result<Rc<Value>, Error> {
        /* Evaluates a source string into one value */

        let exprs = read::read(&source, &filename.into())?;

        match exprs.as_slice() {
            [x] => self.eval(x),
            xs => new_error!("Can only evaluate one expression at a time, not {}", xs.len()).into()
        }
    }


    /* Repl functions */


    pub fn repl_iteration(&mut self) -> Result<Rc<Value>, Error> {
        /* Performs one iteration of the repl */

        let mut input_string = Self::get_line_from_stdin(true)?;

        while !Self::brackets_are_balanced(&input_string)? {
            input_string = format!("{}{}", input_string, Self::get_line_from_stdin(false)?);
        }

        self.eval_source_string(&input_string, "<repl>")
    }


    fn get_line_from_stdin(display_prompt: bool) -> Result<String, Error> {
        /* Reads a line from stdin */

        let mut input_string = String::new();

        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
    
        if display_prompt {
            print!("> ");
        } else {
            print!("  ");
        }

        if let Err(_) = stdout.flush() {
            return new_error!("failed to flush stdout").into();
        }
    
        if let Err(_) = stdin.read_line(&mut input_string) {
            return new_error!("failed to read line from stdin").into();
        }

        Ok(input_string)
    }


    fn brackets_are_balanced(string: &String) -> Result<bool, Error> {
        /* Returns whether a string has balanced brackets */

        let mut bracket_depth = 0;

        for c in string.chars() {
            match c {
                '('|'['|'{' => bracket_depth += 1,
                ')'|']'|'}' => bracket_depth -= 1,
                _ => {}
            }
        }

        if bracket_depth < 0 {
            new_error!("input string has more closing braces than opening braces").into()
        } else {
            Ok(bracket_depth == 0)
        }
    }


    /* Non-built-in function evaluation */

    fn evaluate_lambda_funcall(&self, function: &Rc<Value>, arg_values: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Evaluates the calling of a non-built-in function */

        let function_components = match self.resolve(function)?.to_list() {
            Some(xs) => xs,
            None => return new_error!("Liszp: function should have syntax (lambda <args> <body>)").into()
        };

        if function_components.len() != 3 {
            return new_error!("Liszp: function should have syntax (lambda <args> <body>)").into();
        } else if function_components[0].name() != "&lambda" {
            return new_error!("Liszp: attempt to call a non-function value").into();
        }

        let arg_names = Self::get_arg_names(&function_components[1])?;
        let mut arg_map = Self::build_argument_hashmap(&arg_names, arg_values)?;

        let function_body = &function_components[2];

        self.recursively_bind_args(function_body, &mut arg_map)
    }


    fn get_arg_names(arg_component: &Rc<Value>) -> Result<Vec<String>, Error> {
        /* Gets the list of argument names from the argument component */

        match &**arg_component {
            Value::Cons {..} => {
                let values_list = arg_component.to_list().unwrap();
                let mut names = Vec::with_capacity(values_list.len());

                for v in values_list.iter() {
                    match &**v {
                        Value::Name(name) => names.push(name.clone()),
                        _ => return new_error!("Liszp: Expected name in function argument").into()
                    }
                }

                Ok(names)
            }

            Value::Name(name) => {
                Ok(vec![ name.clone() ])
            }

            Value::Nil => Ok(vec![]),

            _ => return new_error!("Liszp: Function expected a list of arguments or a single argument in lambda expression").into()
        }
    }


    fn build_argument_hashmap(arg_names: &Vec<String>, arg_values: &Vec<Rc<Value>>) -> Result<ValueMap, Error> {
        /* Builds a map from argument names to argument values */

        let mut hashmap = HashMap::new();

        if arg_names.len() != arg_values.len() {
            return new_error!("Function takes {} arguments but received {}", arg_names.len(), arg_values.len()).into();
        }

        for i in 0..arg_names.len() {
            hashmap.insert(arg_names[i].clone(), arg_values[i].clone());
        }

        Ok(hashmap)
    }


    fn recursively_bind_args(&self, expr: &Rc<Value>, arg_map: &mut ValueMap) -> Result<Rc<Value>, Error> {
        /* Returns function_body but with argument names replaced with their values */

        match &**expr {
            Value::Name(name) => {
                if let Some(value) = arg_map.get(name) {
                    Ok(value.clone())
                } else {
                    Ok(expr.clone())
                }
            },

            Value::Cons { car, cdr } => {
                if car.name() == "&lambda" {
                    let lambda_components = match expr.to_list() {
                        Some(xs) => xs,
                        _ => return new_error!("malformed lambda expression").into()
                    };

                    let arg_component = &lambda_components[1];
                    let body_component = &lambda_components[2];

                    let shadowed_arguments = Self::remove_shadowed_arguments(arg_component, arg_map)?;

                    let body_with_bound_arguments = self.recursively_bind_args(body_component, arg_map);

                    arg_map.extend(shadowed_arguments);

                    Ok(refcount_list![
                        lambda_components[0].clone(),
                        arg_component.clone(),
                        body_with_bound_arguments?
                    ])
                } else {
                    Ok(Rc::new(Value::Cons {
                        car: self.recursively_bind_args(car, arg_map)?,
                        cdr: self.recursively_bind_args(cdr, arg_map)?
                    }))
                }
            }

            _ => Ok(expr.clone())
        }
    }


    fn remove_shadowed_arguments(arg_component: &Rc<Value>, arg_map: &mut ValueMap) -> Result<ValueMap, Error> {
        /* Removes any arguments from arg_map that are shadowed in lambda_components */

        let mut shadowed_args = HashMap::new();

        for arg_name in Self::get_arg_names(arg_component)? {
            if let Some(removed_value) = arg_map.remove(&arg_name) {
                shadowed_args.insert(arg_name, removed_value);
            }
        }

        Ok(shadowed_args)
    }


    /* built-in functions */

    fn car(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Gets the car of a cons pair */

        match args.as_slice() {
            [continuation, xs] => {
                let resolved = self.resolve(xs)?;

                let xs = match &*resolved {
                    Value::Quote(cons) => cons.clone(),
                    _ => unreachable!()
                };

                let car = match &*xs {
                    Value::Cons { car, .. } => car,
                    _ => return new_error!("Liszp: function 'cons' expected to receive cons pair").into()
                };

                // If car is a name or cons pair, we must quote it again
                let potentially_quoted_car = match &**car {
                    Value::Cons {..} => Value::Quote(car.clone()).rc(),
                    Value::Name(_)   => Value::Quote(car.clone()).rc(),
                    _                => car.clone()
                };

                Ok(refcount_list![ continuation, &potentially_quoted_car ])
            },

            _ => new_error!("Liszp: function 'car' takes 1 argument").into()
        }
    }


    fn cdr(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Gets the cdr of a cons pair */

        match args.as_slice() {
            [continuation, xs] => {
                let resolved = self.resolve(xs)?;

                let xs = match &*resolved {
                    Value::Quote(cons) => cons.clone(),
                    _ => unreachable!()
                };

                let cdr = match &*xs {
                    Value::Cons { cdr, .. } => cdr,
                    _ => return new_error!("Liszp: function 'cons' expected to receive cons pair").into()
                };

                // If cdr is a name or cons pair, we must quote it again
                let potentially_quoted_cdr = match &**cdr {
                    Value::Cons {..} => Value::Quote(cdr.clone()).rc(),
                    Value::Name(_)   => Value::Quote(cdr.clone()).rc(),
                    _                => cdr.clone()
                };

                Ok(refcount_list![ continuation, &potentially_quoted_cdr ])
            },

            _ => new_error!("Liszp: function 'cdr' takes 1 argument").into()
        }
    }


    fn cons(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Creates a cons pair */

        match args.as_slice() {
            [continuation, car, cdr] => {
                let car = self.resolve(car)?;
                let cdr = self.resolve(cdr)?;

                let cons_pair = Value::Quote(
                    Rc::new(Value::Cons {
                        car: if let Value::Quote(v) = &*car {
                            v.clone()
                        } else {
                            car
                        },

                        cdr: if let Value::Quote(v) = &*cdr {
                            v.clone()
                        } else {
                            cdr
                        }
                    })
                );

                Ok(refcount_list![ continuation.clone(), cons_pair.rc() ])
            }

            _ => new_error!("Liszp: function 'cons' expected 2 arguments").into()
        }
    }


    fn define_value(&mut self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Defines a value in self.globals */

        if args.len() != 3 {
            return new_error!("Liszp: expected syntax (def <name> <value>)").into();
        }

        let continuation = &args[0];
        let name = &args[1];
        let value = &args[2];

        if let Value::Name(name) = &**name {
            self.globals.insert(name.clone(), value.clone());
        } else {
            return new_error!("Liszp: expected name in def expression").into();
        }

        Ok(refcount_list![ continuation.clone(), Value::Nil.rc() ])
    }


    fn eval_quoted(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Evaluates a quoted value */

        match args.as_slice() {
            [continuation, quoted_value] => {
                let value = match &*self.resolve(quoted_value)? {
                    Value::Quote(v) => {
                        CPSConverter::convert_expr_with_continuation(v, continuation)?
                    },

                    _ => quoted_value.clone()
                };

                Ok(refcount_list![ continuation, &value ])
            }

            _ => new_error!("Liszp: function 'quote' takes exactly one argument").into()
        }
    }


    fn if_expr(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Evaluates an if expression */
    
        if args.len() != 3 {
            return new_error!("Liszp: if expression has syntax (if <condition> <true case> <false case>)").into();
        }

        let cond = self.resolve(&args[0])?;
        let true_case = self.resolve(&args[1])?;
        let false_case = self.resolve(&args[2])?;

        if let Value::Bool(b) = &*cond {
            if *b {
                Ok(true_case)
            } else {
                Ok(false_case)
            }
        } else {
            new_error!("if expression expected a boolean condition").into()
        }
    }


    fn no_continuation(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* The final stage of a trampolined evaluation */

        if args.len() == 1 {
            self.resolve(&args[0])
        } else {
            unreachable!()
        }
    }


    fn panic(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Panics */

        match args.as_slice() {
            [_, msg] => panic!("{}", msg),
            _ => new_error!("Liszp: expected syntax (panic <message>)").into()
        }
    }


    fn print_value(&self, args: &Vec<Rc<Value>>, newline: bool) -> Result<Rc<Value>, Error> {
        /* Prints a value, optionally with a newline */

        if args.len() != 2 {
            return new_error!("Function print{} takes 1 argument only", if newline { "ln" } else { "" }).into();
        }

        let continuation = &args[0];
        let value = self.resolve(&args[1])?;

        if newline {
            println!("{}", value);
        } else {
            print!("{}", value);
        }

        Ok(refcount_list![ continuation.clone(), value])
    }


    fn quote_value(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Quotes a value */

        match args.as_slice() {
            [continuation, value] => {
                let quoted_value = match &**value {
                    Value::Quote(_) => value.clone(),
                    _ => Value::Quote(self.resolve(value)?).rc()
                };

                Ok(refcount_list![ continuation, &quoted_value ])
            }

            _ => new_error!("Liszp: function 'quote' takes exactly one value").into()
        }
    }


    fn values_are_equal(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Compares two values */

        match args.as_slice() {
            [continuation, x, y] => {
                let result = Value::Bool(self.resolve(x)? == self.resolve(y)?).rc();

                Ok(refcount_list![ continuation, &result ])
            },

            _ => new_error!("Liszp: Function 'equals?' takes exactly 2 parameters").into()
        }
    }


    fn value_is_bool(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Returns whether a value is a bool */

        match args.as_slice() {
            [continuation, value] => {
                let resolved = self.resolve(value)?;

                let value = match &*resolved {
                    Value::Quote(v) => v,
                    _ => &resolved
                };

                let result = match &**value {
                    Value::Bool(_) => true,
                    _ => false
                };

                Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
            },

            _ => new_error!("Liszp: function 'bool?' takes exactly one argument").into()
        }
    }


    fn value_is_cons(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Returns whether a value is a cons pair */

        match args.as_slice() {
            [continuation, value] => {
                let resolved = self.resolve(value)?;

                let value = match &*resolved {
                    Value::Quote(v) => v,
                    _ => &resolved
                };

                let result = match &**value {
                    Value::Cons {..} => true,
                    _ => false
                };

                Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
            },

            _ => new_error!("Liszp: function 'cons?' takes exactly one argument").into()
        }
    }


    fn value_is_float(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {   
        /* Returns whether a value is a float */

        match args.as_slice() {
            [continuation, value] => {
                let resolved = self.resolve(value)?;

                let value = match &*resolved {
                    Value::Quote(v) => v,
                    _ => &resolved
                };

                let result = match &**value {
                    Value::Float(_) => true,
                    _ => false
                };

                Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
            },

            _ => new_error!("Liszp: function 'float?' takes exactly one argument").into()
        }
    }


    fn value_is_int(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Returns whether a value is an int */

        match args.as_slice() {
            [continuation, value] => {
                let resolved = self.resolve(value)?;

                let value = match &*resolved {
                    Value::Quote(v) => v,
                    _ => &resolved
                };

                let result = match &**value {
                    Value::Integer(_) => true,
                    _ => false
                };

                Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
            },

            _ => new_error!("Liszp: function 'int?' takes exactly one argument").into()
        }
    }


    fn value_is_nil(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Returns whether a value is nil */

        match args.as_slice() {
            [continuation, value] => {
                let resolved = self.resolve(value)?;

                let value = match &*resolved {
                    Value::Quote(v) => v,
                    _ => &resolved
                };

                let result = match &**value {
                    Value::Nil => true,
                    _ => false
                };

                Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
            },

            _ => new_error!("Liszp: function 'nil?' takes exactly one argument").into()
        }
    }


    fn value_is_name(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Returns whether a value is a name */

        match args.as_slice() {
            [continuation, value] => {
                let value = match &**value {
                    Value::Quote(v) => v,
                    _ => value
                };

                let result = match &**value {
                    Value::Name(_) => true,
                    _ => false
                };

                Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
            },

            _ => new_error!("Liszp: function 'name?' takes exactly one argument").into()
        }
    }


    fn value_is_quote(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Returns whether a value is quoted */

        match args.as_slice() {
            [continuation, value] => {
                let result = match &*self.resolve(value)? {
                    Value::Quote(_) => true,
                    _ => false
                };

                Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
            },

            _ => new_error!("Liszp: function 'quote?' takes exactly one argument").into()
        }
    }


    fn value_is_str(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Returns whether a value is a str */

        match args.as_slice() {
            [continuation, value] => {
                let resolved = self.resolve(value)?;

                let value = match &*resolved {
                    Value::Quote(v) => v,
                    _ => &resolved
                };

                let result = match &**value {
                    Value::String(_) => true,
                    _ => false
                };

                Ok(refcount_list![ continuation.clone(), Value::Bool(result).rc() ])
            },

            _ => new_error!("Liszp: function 'str?' takes exactly one argument").into()
        }
    }


    /* Arithmetic */

    fn arithmetic_expression(&self, op: &String, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Computes an arithmetic expression */

        if args.len() < 2 {
            return new_error!("Liszp: '{}' expression takes at least 1 argument", op).into();
        }

        let mut numbers = Vec::with_capacity(args.len());
        let continuation = &args[0];
        let mut result_is_float = false;

        for arg in args.iter().dropping(1) {
            let arg = self.resolve(arg)?;

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
            Self::float_arithmetic(op, &numbers)
        } else {
            Self::integer_arithmetic(op, &numbers)
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


    fn modulo(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Takes the modulus of a number */

        match args.as_slice() {
            [continuation, dividend, divisor] => {
                let dividend = self.resolve(dividend)?;
                let divisor = self.resolve(divisor)?;

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

    fn binary_logical_operation(&self, op: &String, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Evaluates a binary logical operation */

        match args.as_slice() {
            [continuation, x, y] => {
                let x = match &*self.resolve(x)? {
                    Value::Bool(b) => *b,
                    _ => return new_error!("Liszp: {} expressions take boolean arguments", &op[1..]).into()
                };

                let y = match &*self.resolve(y)? {
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


    fn logical_negation(&self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Performs a logical not operation */

        match args.as_slice() {
            [continuation, x] => {
                let x = match &*self.resolve(x)? {
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

    fn comparison(&self, op: &String, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Compares two values */

        match args.as_slice() {
            [continuation, x, y] => {
                let x = self.resolve(x)?;
                let y = self.resolve(y)?;

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
}
