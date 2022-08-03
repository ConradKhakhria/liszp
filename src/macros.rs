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


#[derive(Clone)]
pub struct Macro {
    name: Rc<Value>,
    args: MacroArgs,
    macro_as_function: Rc<Value>
}


#[derive(Clone)]
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


    fn parse_macro_definition(expr: &Rc<Value>) -> Result<Option<Self>, Error> {
       /* Parses a macro definition if one is defined in expr
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

        let signature_components = match components[1].to_list() {
            Some(xs) => xs,
            None => return new_error!("expected the macro signature to be a list (<name> <args>..)").into()
        };

        if signature_components.is_empty(){
            return new_error!("A macro definition cannot have an empty signature").into();   
        }

        let macro_name = match &*signature_components[0] {
            Value::Name(_) => signature_components[0].clone(),
            _ => return new_error!("expected name in macro definition").into()
        };

        let macro_args = Self::parse_macro_args(macro_name.name(), &signature_components[1..])?;

        let macro_as_function = Self::macro_as_function(&macro_args, &components[2]);

        Ok(Some(
            Macro {
                name: macro_name,
                args: macro_args,
                macro_as_function: Evaluator::parse_lambdas(&macro_as_function)?
            }
        ))
    }


    fn macro_as_function(macro_args: &MacroArgs, body_component: &Rc<Value>) -> Rc<Value> {
        /* Creates an executable function out of a macro */

        let function_args = match macro_args {
            MacroArgs::Finite(xs) => xs,
            MacroArgs::Variadic { arg_names, .. } => arg_names
        };

        refcount_list![
            Value::Name("lambda".into()).rc(),
            function_args.clone(),
            body_component.clone()
        ]
    }



    /* Macro expansion */


    fn expand_macro(&self, components: &Vec<Rc<Value>>, evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
        /* Expands a macro in an expression */

        // temporarily remove self from the evaluator's macro namespace
        let old_self = evaluator.get_macros().remove(&self.name.name()).unwrap();

        // add macro as function to env
        evaluator.get_env().insert(self.name.name(), self.macro_as_function.clone());

        let quoted_args = self.generate_quoted_args(components[1..].to_vec())?;
        let executable_expr = Value::Cons {
            car: self.name.clone(),
            cdr: Value::cons_list(&quoted_args)
        }.rc();

        let evaluation_result = evaluator.eval(&executable_expr)?;

        // remove macro as function from env
        evaluator.get_env().remove(&self.name.name());

        // return self to the macro namespace
        evaluator.get_macros().insert(self.name.name(), old_self);

        Ok(evaluation_result)
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

    if let Some(new_macro) = Macro::parse_macro_definition(expr)? {
        let new_macro_name = new_macro.name.name();

        if evaluator.get_macros().insert(new_macro_name.clone(), new_macro).is_some() {
            return new_error!("macro '{}' has already been defined", new_macro_name).into()
        } else {
            return Ok(Value::Nil.rc())
        }
    }

    let components = match expr.to_list() {
        Some(xs) => xs,
        None => return Ok(expr.clone())
    };

    if components.is_empty() {
        return Ok(expr.clone());
    }

    match evaluator.get_macros().get(&components[0].name()) {
        Some(m) => m.clone().expand_macro(&components, evaluator),

        None => {
            let mut new_components = vec![];

            for comp in components.iter() {
               new_components.push(recursively_expand_macros(comp, evaluator)?);
            }

            Ok(Value::cons_list(&new_components))
        }
    }
}
