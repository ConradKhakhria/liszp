use crate::read;
use crate::error::Error;
use crate::eval::{ builtin, operators };
use crate::new_error;
use crate::macros;
use crate::value::Value;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;


type ValueMap = HashMap<String, Rc<Value>>;

pub struct Evaluator {
    evaluated: Vec<Rc<Value>>,
    env: ValueMap,
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

        let macro_expanded = macros::expand_macros(expr, self)?;

        Ok(macro_expanded)
    }


    /* Eval */


    pub fn eval(&mut self, expr: &Rc<Value>) -> Result<Rc<Value>, Error> {
        /* Evaluates an expression */

        let value = self.preprocess(expr)?;

        println!("{}", &value);

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


    /* function evaluation */


    fn evaluate_lambda_funcall(&mut self, function: &Rc<Value>, arg_values: &Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Evaluates the calling of a non-built-in function */

        let function_components = match self.eval(&function)?.to_list() {
            Some(xs) => xs,
            None => return new_error!("value '{}' is not a function", function).into()
        };


        match function_components.as_slice() {
            [kwd_lambda, args, body] => {
                if kwd_lambda.name() != "lambda" {
                    return new_error!("Liszp: attempt to call a non-function value").into();
                }

                let arg_names = Self::get_arg_names(args)?;
                let replaced_values = self.add_args_to_env(&arg_names, arg_values)?;

                let result = self.eval(body);

                self.replace_old_values(&replaced_values);

                result
            }

            _ => new_error!("function should have syntax (lambda <args> <body>)").into()
        }
    }


    fn add_args_to_env(&mut self, arg_names: &Vec<String>, arg_values: &Vec<Rc<Value>>) -> Result<ValueMap, Error> {
        /* Adds the values */

        if arg_names.len() != arg_values.len() {
            return new_error!("function expected {} arguments but received {}", arg_names.len(), arg_values.len()).into();
        }
        
        let mut replaced_values = HashMap::new();

        for i in 0..arg_names.len() {
            if let Some(old_value) = self.env.insert(arg_names[i].clone(), arg_values[i].clone()) {
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
}
