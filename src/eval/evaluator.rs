use crate::read;
use crate::error::Error;
use crate::eval::{ builtin, operators };
use crate::new_error;
use crate::preprocess::macros;
use crate::refcount_list;
use crate::value::Value;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;


type ValueMap = HashMap<String, Rc<Value>>;

pub struct Evaluator {
    evaluated: Vec<Rc<Value>>,
    globals: ValueMap,
    pub macros: HashMap<String, macros::Macro>,
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


    pub fn resolve(&self, value: &Rc<Value>) -> Result<Rc<Value>, Error> {
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


    /* Preprocessing */


    fn preprocess(&mut self, expr: &Rc<Value>) -> Result<Rc<Value>, Error> {
        /* Preprocesses an expression */

        if let Some(ref macro_expanded) = macros::expand_macros(expr, self)? {
            let formatted = crate::preprocess::fmt::format_names(macro_expanded);
            let cps_converted = crate::preprocess::cps::convert_expr(&formatted)?;

            Ok(cps_converted)
        } else {
            Ok(Value::Nil.rc())
        }
    }


    /* Eval */


    pub fn eval(&mut self, expr: &Rc<Value>) -> Result<Rc<Value>, Error> {
        /* Evaluates an expression in Env */

        let mut value = self.preprocess(expr)?;

        while let Value::Cons { car: function, cdr: args  } = &*value {
            let function_name = function.name();
            let args = args.to_list().expect("Liszp: expected a list of arguments");

            value = match function_name.as_str() {
                "&bool?"            => builtin::value_is_bool(&args, self)?,
                "&car"              => builtin::car(&args, self)?,
                "&cdr"              => builtin::cdr(&args, self)?,
                "&cons"             => builtin::cons(&args, self)?,
                "&cons?"            => builtin::value_is_cons(&args, self)?,
                "&def"              => self.define_value(&args)?,
                "&equals?"          => builtin::values_are_equal(&args, self)?,
                "&eval"             => builtin::eval_quoted(&args, self)?,
                "&float"            => builtin::value_is_float(&args, self)?,
                "&if"               => builtin::if_expr(&args, self)?,
                "&int?"             => builtin::value_is_int(&args, self)?,
                "&name?"            => builtin::value_is_name(&args)?,
                "&nil?"             => builtin::value_is_nil(&args, self)?,
                "&panic"            => builtin::panic(&args)?,
                "&print"            => builtin::print_value(&args, self, false)?,
                "&println"          => builtin::print_value(&args, self, true)?,
                "&quote"            => builtin::quote_value(&args, self)?,
                "&quote?"           => builtin::value_is_quote(&args, self)?,
                "&str?"             => builtin::value_is_str(&args, self)?,
                "&+"|"&-"|"&*"|"&/" => operators::arithmetic_expression(&function_name, &args, self)?,
                "&%"                => operators::modulo(&args, self)?,
                "&and"|"&or"|"&xor" => operators::binary_logical_operation(&function_name, &args, self)?,
                "&not"              => operators::logical_negation(&args, self)?,
                "&<"|"&>"|"&<="|
                "&>="|"&=="|"&!="   => operators::comparison(&function_name, &args, self)?,
                "no-continuation"   => {
                    if args.len() == 1{
                        value = args[0].clone();
                        break;
                    } else {
                        unreachable!()
                    }
                },
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


    /* function evaluation */


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
}
