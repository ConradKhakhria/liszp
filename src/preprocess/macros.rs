use crate::{
    error::Error,
    eval::Evaluator,
    new_error,
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
    fn to_function(&self) -> Rc<Value> {
        /* Creates a function out of the macro */

        refcount_list![
            Value::Name("&lambda".into()).rc(),
            self.args.clone(),
            self.body.clone()
        ]
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


    fn expand_macros(&mut self, value: &Rc<Value>) -> Result<Rc<Value>, Error> {
        /* Returns value but with all macros expanded */

        todo!()
    }


    fn parse_macro_definition(&mut self, expr: &Rc<Value>) -> Result<Option<Macro>, Error> {
        /* Attempts to parse a macro definition */

        let components = match expr.to_list() {
            Some(xs) => xs,
            None => unreachable!()
        };

        if components.is_empty() || components[0].name() != "&defmacro" {
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
                let macro_expanded = macro_expander.expand_macros(value)?;
                macro_expanded_values.push(macro_expanded);
            }
        }
    }

    Ok(macro_expanded_values)
}
