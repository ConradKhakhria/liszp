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
    name: String,
    args: Vec<String>,
    body: Rc<Value>
}


impl Macro {
    fn to_function(&self) -> Rc<Value> {
        /* Creates a function out of the macro */

        let mut args = vec![];

        for arg in self.args.iter() {
            args.push(Value::Name(arg.to_string()).rc())
        }

        refcount_list![
            Value::Name("&lambda".into()).rc(),
            Value::cons_list(&args),
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

        let macro_name = m.name.clone();

        match self.macros.insert(macro_name.clone(), m) {
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

        todo!()
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
