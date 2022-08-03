/* This module is currently parked 
 *
 * While the evaluator is reconfigured to include preprocessing,
 * This module will exist but not be used.
 */

use crate::error::Error;
use crate::eval::Evaluator;
use crate::new_error;
use crate::refcount_list;
use crate::value::Value;
use std::rc::Rc;


/* Macro struct */


pub struct Macro {
    name: Rc<Value>,
    args: MacroArgs,
    body: Rc<Value>
}


enum MacroArgs {
    Finite(Rc<Value>),

    Variadic {
        arg_names: Rc<Value>,
        named_args_count: usize,
        variadic_name: Rc<Value>
    }
}


impl Macro {

    /* Parsing macro definitions */

    fn from_expr(expr: &Rc<Value>) -> Result<Option<Macro>, Error> {
       /* Attempts to parse a macro definition
        *
        * returns
        * -------
        * - Err(error)  if an error occurs
        * - Ok(None)    if the expr is not a macro definition
        * - Ok(Some(m)) if the expr is a macro definition
        */

        let components = match expr.to_list() {
            Some(xs) => xs,
            None => return Ok(None)
        };
    
        if components.is_empty() || components[0].name() != "defmacro" {
            return Ok(None);
        }
    
        if components.len() != 3 {
            return new_error!("expected syntax (defmacro <macro-signature> <macro-body>").into();
        }

        let (name, args) = Self::parse_macro_signature(&components[1])?;

        Ok(Some(
            Macro {
                name,
                args,
                body: components[2].clone()
            }
        ))
    }


    fn parse_macro_signature(expr: &Rc<Value>) -> Result<(Rc<Value>, MacroArgs), Error> {
        /* Parses the name and arg names of a macro */

        let signature_components = match expr.to_list() {
            Some(xs) => xs,
            None => return new_error!("expected the macro signature to be a list (<name> <args>..)").into()
        };
    
        if signature_components.is_empty() {
            return new_error!("A macro definition cannot have an empty signature").into();
        }

        let name = signature_components[0].clone();

        if name.name() == "" {
            return new_error!("macro definition expects an identifier for a name").into();
        }

        let args = Self::parse_macro_args(name.name(), &signature_components[1..])?;

        Ok((name, args))
    }


    fn parse_macro_args(macro_name: String, arg_components: &[Rc<Value>]) -> Result<MacroArgs, Error> {
        /* Pareses a macro signature's arguments */

        let mut named_args = vec![];

        for i in 0..arg_components.len() {
            match &*arg_components[i] {
                Value::Name(name) => {
                    if name == "@" {
                        if i + 2 != arg_components.len() {
                            return new_error!("macros with variadic parameters can only have one vararg").into();
                        }

                        if let Value::Name(_) = &*arg_components[i + 1] {
                            named_args.push(arg_components[i].clone());

                            let macro_args = MacroArgs::Variadic {
                                arg_names: Value::cons_list(&named_args),
                                named_args_count: named_args.len(),
                                variadic_name: arg_components[i + 1].clone()
                            };

                            return Ok(macro_args);
                        } else {
                            return new_error!("Expected name for vararg").into();
                        }
                    } else {
                        named_args.push(arg_components[i].clone());
                    }
                },

                _ => return new_error!("in '{}' macro definition: expected name in macro argument", macro_name).into()
            }
        }

        Ok(MacroArgs::Finite(Value::cons_list(&named_args)))
    }


    /* Misc */

    fn to_executable_expression(&self, supplied_args: Vec<Rc<Value>>) -> Result<Rc<Value>, Error> {
        /* Creates an executable expression from self and supplied arguments */

        let quoted_supplied_args = self.generate_quoted_args(supplied_args)?;

        let defined_arg_names = match &self.args {
            MacroArgs::Finite(xs) => xs.clone(),
            MacroArgs::Variadic { variadic_name, .. } => variadic_name.clone()
        };

        let macro_as_function = refcount_list![
            Value::Name("lambda".into()).rc(),
            defined_arg_names,
            self.body.clone()
        ];

        Ok(Rc::new(
            Value::Cons {
                car: macro_as_function,
                cdr: Value::cons_list(&quoted_supplied_args)
            }
        ))
    }


    fn generate_quoted_args(&self, supplied_args: Vec<Rc<Value>>) -> Result<Vec<Rc<Value>>, Error> {
        /* list of args suppled to macro -> args to supply to lambda */

        let mut quoted_supplied_args = Vec::with_capacity(supplied_args.len());
        let quote_name = Value::Name("quote".into()).rc();

        match &self.args {
            MacroArgs::Finite(_) => {
                for arg in supplied_args.iter() {
                    quoted_supplied_args.push(refcount_list![ quote_name.clone(), arg.clone() ]);
                }
            },

            MacroArgs::Variadic { named_args_count, .. } => {
                if *named_args_count > supplied_args.len() {
                    return new_error!("macro '{}' invoked with too few arguments", self.name.name()).into();
                }

                for i in 0..(*named_args_count - 1) {
                    quoted_supplied_args.push(refcount_list![ quote_name.clone(), supplied_args[i].clone() ]);
                }

                let mut quoted_supplied_varargs = vec![ Value::Name("list".into()).rc() ];

                for arg in supplied_args[*named_args_count - 1..].iter() {
                    quoted_supplied_varargs.push(refcount_list![ quote_name.clone(), arg.clone() ]);
                }

                quoted_supplied_args.push(Value::cons_list(&quoted_supplied_varargs));
            }
        }

        Ok(quoted_supplied_args)
    }
}


pub fn recursively_expand_macros(expr: &Rc<Value>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Expands all macros in an expression */

    match Macro::from_expr(expr)? {
        Some(new_macro) => {
            let new_macro_name = new_macro.name.name();

            if evaluator.macros.insert(new_macro_name.clone(), new_macro).is_some() {
                new_error!("macro '{}' has already been defined", new_macro_name).into()
            } else {
                Ok(Value::Nil.rc())
            }
        },

        None => {
            let components = match expr.to_list() {
                Some(xs) => xs,
                None => return Ok(expr.clone())
            };

            if components.is_empty() {
                return Ok(expr.clone());
            }

            match evaluator.macros.get(&components[0].name()) {
                Some(m) => {
                    let supplied_args = components[1..].to_vec();
                    let executable_expr = m.to_executable_expression(supplied_args)?;

                    evaluator.eval(&executable_expr)
                },

                None => {
                    let mut new_components = vec![];

                    for comp in components.iter() {
                       new_components.push(recursively_expand_macros(comp, evaluator)?);
                    }

                    Ok(Value::cons_list(&new_components))
                }
            }
        }
    }
}
