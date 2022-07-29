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
    args: Rc<Value>,
    body: Rc<Value>
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

        let [name, args] = Self::parse_macro_signature(&components[1])?;

        Ok(Some(
            Macro {
                name,
                args,
                body: components[2].clone()
            }
        ))
    }


    fn parse_macro_signature(expr: &Rc<Value>) -> Result<[Rc<Value>; 2], Error> {
        /* Parses the name and arg names of a macro */

        let signature_components = match expr.to_list() {
            Some(xs) => xs,
            None => return new_error!("expected the macro signature to be a list (<name> <args>..)").into()
        };
    
        for comp in signature_components.iter() {
            match &**comp {
                Value::Name(_) => {},
                _ => return new_error!("the macro signature should consist only of names").into()
            }
        }

        match &*Value::cons_list(&signature_components) {
            Value::Cons { car, cdr } => Ok([car.clone(), cdr.clone()]),
            _ => unreachable!()
        }
    }


    /* Misc */

    fn to_executable_expression(&self, supplied_args: Vec<Rc<Value>>) -> Rc<Value> {
        /* Creates an executable expression from self and supplied arguments */

        let macro_as_function = refcount_list![
            Value::Name("lambda".into()).rc(),
            self.args.clone(),
            self.body.clone()
        ];

        let mut quoted_supplied_args = Vec::with_capacity(supplied_args.len());
        let quote_name = Value::Name("quote".into()).rc();

        for arg in supplied_args.iter() {
            quoted_supplied_args.push(refcount_list![ quote_name.clone(), arg.clone() ])
        }

        Value::Cons {
            car: macro_as_function,
            cdr: Value::cons_list(&quoted_supplied_args)
        }.rc()
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
                    let executable_expr = m.to_executable_expression(supplied_args);

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
