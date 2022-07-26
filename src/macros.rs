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
    fn to_executable_expression(&self, supplied_args: &[Rc<Value>]) -> Rc<Value> {
        /* Creates an executable expression from self and supplied arguments */

        let macro_as_function = refcount_list![
            Value::Name("lambda".into()).rc(),
            self.args.clone(),
            self.body.clone()
        ];

        let mut quoted_args = Vec::with_capacity(supplied_args.len());

        for arg in supplied_args.iter() {
            quoted_args.push(Value::Quote(arg.clone()).rc());
        }

        Value::Cons {
            car: macro_as_function,
            cdr: Value::cons_list(&quoted_args)
        }.rc()
    }
}


fn add_macro(m: Macro, evaluator: &mut Evaluator) -> Result<(), Error> {
    /* Adds a macro to the scope */

    let macro_name = m.name.name();

    match evaluator.macros.insert(m.name.name(), m) {
        Some(_) => new_error!("macro '{}' has already been defined", macro_name).into(),
        None => Ok(())
    }
}


pub fn expand_macros(expr: &Rc<Value>, evaluator: &mut Evaluator) -> Result<Option<Rc<Value>>, Error> {
    /* Expands all macros in an expression
     *
     * Returns
     * -------
     * - Err(..)      : an error, if one occurs
     * - Ok(None)     : if expr is a macro defintion
     * - Ok(Some(..)) : an expression with all macros expanded
     */

     if let Some(new_macro) = parse_macro_definition(expr)? {
         add_macro(new_macro, evaluator)?;

         return Ok(None);
     }

     match expr.to_list() {
         Some(components) => {
             if components.is_empty() {
                return Ok(Some(expr.clone()));
             }

             match evaluator.macros.get(&components[0].name()) {
                 Some(m) => {
                     let supplied_args = &components[1..];
                     let executable_expression = m.to_executable_expression(supplied_args);

                     evaluator.eval(&executable_expression)
                              .map(|v| Some(v.clone()))
                 }

                 None => {
                     let mut new_components = vec![];

                     for comp in components.iter() {
                         match expand_macros(comp, evaluator)? {
                             Some(v) => new_components.push(v),
                             None => return new_error!("Cannot define a macro inside an expression").into()
                         }
                     }

                     Ok(Some(Value::cons_list(&new_components)))
                 }
             }
         }

         None => Ok(Some(expr.clone()))
     }
 }


 fn parse_macro_definition(expr: &Rc<Value>) -> Result<Option<Macro>, Error> {
    /* Attempts to parse a macro definition */

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

    /* Parse args */

    let signature_components = match components[1].to_list() {
        Some(xs) => xs,
        None => return new_error!("expected the macro signature to be a list (<name> <args>..)").into()
    };

    for comp in signature_components.iter() {
        match &**comp {
            Value::Name(_) => {},
            _ => return new_error!("the macro signature should consist only of names").into()
        }
    }

    let (name, args) = match &*Value::cons_list(&signature_components) {
        Value::Cons { car, cdr } => (car.clone(), cdr.clone()),
        _ => unreachable!()
    };

    /* Parse body */

    let body = components[2].clone();

    Ok(Some(
        Macro {
            name,
            args,
            body
        }
    ))
}
