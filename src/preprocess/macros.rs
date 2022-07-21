use crate::{
    error::Error,
    eval::Evaluator,
    new_error,
    preprocess::cps,
    refcount_list,
    value::Value
};

use std::{
    collections::HashMap,
    rc::Rc
};


/* Macro struct */

struct Macro {
    name: Rc<Value>,
    args: Rc<Value>,
    body: Rc<Value>
}


impl Macro {
    fn to_executable_expression(&self, supplied_args: &[Rc<Value>]) -> Rc<Value> {
        /* Creates an executable expression from self and supplied arguments */

        let macro_as_function = refcount_list![
            Value::Name("&lambda".into()).rc(),
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


/* Macro expander */

pub struct MacroExpander {
    evaluator: Evaluator,
    macros: HashMap<String, Macro>,
}


impl MacroExpander {
    pub fn new() -> Self {
        /* Creates a new MacroExpander */

        MacroExpander {
            evaluator: Evaluator::new(),
            macros: HashMap::new()
        }
    }


    fn add_macro(&mut self, m: Macro) -> Result<(), Error> {
        /* Adds a macro to the scope */

        let macro_name = m.name.name();

        match self.macros.insert(m.name.name(), m) {
            Some(_) => new_error!("macro '{}' has already been defined", macro_name).into(),
            None => Ok(())
        }
    }


    fn expand_macros_in_expression(&mut self, value: &Rc<Value>) -> Result<Rc<Value>, Error> {
        /* Returns value but with all macros expanded */

        match value.to_list() {
            Some(components) => {
                if components.is_empty() {
                   return Ok(value.clone());
                }

                match self.macros.get(&components[0].name()) {
                    Some(m) => {
                        todo!()
                    },

                    None => {
                        let mut new_components = vec![];

                        for comp in components.iter() {
                            new_components.push(self.expand_macros_in_expression(comp)?);
                        }

                        Ok(Value::cons_list(&new_components))
                    }
                }
            }

            None => Ok(value.clone())
        }
    }


    fn parse_macro_definition(&mut self, expr: &Rc<Value>) -> Result<Option<Macro>, Error> {
        /* Attempts to parse a macro definition */

        let components = match expr.to_list() {
            Some(xs) => xs,
            None => unreachable!()
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
}


pub fn expand_macros(values: &Vec<Rc<Value>>) -> Result<Vec<Rc<Value>>, Error> {
    /* Expands all macros in a list of exprs */

    let mut macro_expander = MacroExpander::new();
    let mut macro_expanded_values = vec![];

    for value in values.iter() {
        match macro_expander.parse_macro_definition(value)? {
            Some(m) => {
                macro_expander.add_macro(m)?;
            },

            None => {
                let macro_expanded = macro_expander.expand_macros_in_expression(value)?;
                macro_expanded_values.push(macro_expanded);
            }
        }
    }

    Ok(macro_expanded_values)
}
