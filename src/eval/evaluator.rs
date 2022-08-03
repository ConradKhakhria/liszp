use crate::read;
use crate::error::Error;
use crate::eval::{ builtin, operators };
use crate::new_error;
use crate::macros;
use crate::value::Value;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;


type ValueMap = HashMap<String, Rc<Value>>;

pub struct Evaluator {
    pub evaluated: Vec<Rc<Value>>,
    pub env: ValueMap,
    pub macros: HashMap<String, macros::Macro>,
}


impl Evaluator {
    pub fn new() -> Self {
        Evaluator {
            evaluated: vec![],
            env: HashMap::new(),
            macros: HashMap::new(),
        }
    }


    pub fn load_stdlib(&mut self) -> Result<(), Error> {
        /* Loads standard macros and functions into the namespace */

        self.eval_file("liszp-stdlib/std-macros.lzp")?;
        self.eval_file("liszp-stdlib/std-functions.lzp")?;

        Ok(())
    }


    /* Env-related functions */


    fn define_value(&mut self, args: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Defines a value in self.globals */
    
        if args.len() != 2 {
            return new_error!("Liszp: expected syntax (def <name> <value>)").into();
        }

        let name = &args[0];
        let value = self.eval(&args[1])?;
    
        if let Value::Name(name) = &**name {
            self.env.insert(name.clone(), value.clone());
        } else {
            return new_error!("Liszp: expected name in def expression").into();
        }

        Ok(Value::Nil.rc())
    }


    /* Preprocessing */


    pub fn preprocess(&mut self, expr: &Rc<Value>) -> Result<Rc<Value>, Error> {
        /* Preprocesses an expression */

        let macro_expanded = macros::recursively_expand_macros(expr, self)?;
        let parsed_lambdas = Self::parse_lambdas(&macro_expanded)?;

        Ok(parsed_lambdas)
    }


    pub fn parse_lambdas(expr: &Rc<Value>) -> Result<Rc<Value>, Error> {
        /* Searches an expression for lambda exprs and turns them into Value::Lambda's */

        let components = match expr.to_list() {
            Some(xs) => xs,
            None => return Ok(expr.clone())
        };

        if components.is_empty() || components[0].name() != "lambda" {
            return Ok(expr.clone());
        }

        match components.as_slice() {
            [_kwd_lambda, args, body] => {
                let arg_names = Self::get_arg_names(args)?;
                let lambda = Value::Lambda {
                    args: arg_names,
                    body: body.clone()
                };

                Ok(lambda.rc())
            },

            _ => new_error!("lambda expressions take the form (lambda <args> <body>").into()
        }
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

            _ => new_error!("Liszp: Function expected a list of arguments or a single argument in lambda expression").into()
        }
    }


    /* Eval */


    pub fn eval(&mut self, expr: &Rc<Value>) -> Result<Rc<Value>, Error> {
        /* Evaluates an expression */

        let value = self.preprocess(expr)?;

        match &*value {
            Value::Cons { car: function, cdr: args } => {
                let function_name = function.name();
                let args = match args.to_list() {
                    Some(xs) => xs,
                    None => return new_error!("expected a list of args").into()
                };

                match function_name.as_str() {
                    "bool?"          => builtin::value_is_bool(&args, self),
                    "car"            => builtin::car(&args, self),
                    "cdr"            => builtin::cdr(&args, self),
                    "cons"           => builtin::cons(&args, self),
                    "cons?"          => builtin::value_is_cons(&args, self),
                    "def"            => self.define_value(&args),
                    "equals?"        => builtin::values_are_equal(&args, self),
                    "eval"           => builtin::eval_quoted(&args, self),
                    "float"          => builtin::value_is_float(&args, self),
                    "if"             => builtin::if_expr(&args, self),
                    "int?"           => builtin::value_is_int(&args, self),
                    "list"           => builtin::make_list(&args, self),
                    "name?"          => builtin::value_is_name(&args),
                    "nil?"           => builtin::value_is_nil(&args, self),
                    "panic"          => builtin::panic(&args, self),
                    "print"          => builtin::print_value(&args, self, false),
                    "println"        => builtin::print_value(&args, self, true),
                    "quote"          => builtin::quote_value(&args),
                    "str?"           => builtin::value_is_str(&args, self),
                    "+"|"-"|"*"|"/"  => operators::arithmetic_expression(&function_name, &args, self),
                    "%"              => operators::modulo(&args, self),
                    "and"|"or"|"xor" => operators::binary_logical_operation(&function_name, &args, self),
                    "not"            => operators::logical_negation(&args, self),
                    "<"|">"|"<="
                    |">="|"=="|"!="  => operators::comparison(&function_name, &args, self),
                    _                => self.evaluate_lambda_funcall(function, &args)
                }
            },

            Value::Name(name) => {
                match self.env.get(name) {
                    Some(v) => Ok(v.clone()),
                    None => new_error!("value '{}' is undefined", name).into()
                }
            },

            _ => Ok(value.clone())
        }
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


    /* function evaluation */


    fn evaluate_lambda_funcall(&mut self, function: &Rc<Value>, arg_values: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Evaluates the calling of a non-built-in function */

        let evaluated_function = self.eval(&function)?;

        let (arg_names, body) = match &*evaluated_function {
            Value::Lambda { args, body } => (args, body),
            _ => return new_error!("expected function, received {}", function).into()
        };

        let replaced_values = self.add_args_to_env(&arg_names, arg_values)?;

        let result = self.eval(&body);

        self.replace_old_values(&replaced_values);

        result
    }


    fn add_args_to_env(&mut self, arg_names: &Vec<String>, arg_values: &Vec<Rc<Value>>) -> Result<ValueMap, Error> {
        /* Adds the values */

        if arg_names.len() != arg_values.len() {
            return new_error!("function expected {} arguments but received {}", arg_names.len(), arg_values.len()).into();
        }
        
        let mut replaced_values = HashMap::new();

        for i in 0..arg_names.len() {
            let evaluated_arg = self.eval(&arg_values[i])?;

            if let Some(old_value) = self.env.insert(arg_names[i].clone(), evaluated_arg) {
                replaced_values.insert(arg_names[i].clone(), old_value);
            }
        }

        Ok(replaced_values)
    }


    fn replace_old_values(&mut self, replaced_values: &ValueMap) {
        /* Replaces all new values in self.env with these old ones */

        for key in replaced_values.keys() {
            self.env.insert(key.clone(), replaced_values.get(key).unwrap().clone());
        }
    }
}
