use crate::{
    value::Value,
    error::Error,
    eval::Evaluator,
};

use std::{
    collections::HashMap,
    rc::Rc
};


/* Macro struct */

struct Macro<'v> {
    name: &'v String,
    args: Vec<&'v str>,
    body: Rc<Value>
}


impl<'s> Macro<'s> {
    fn to_unevaluated_value(&self, supplied_parameters: &Vec<&'s str>) -> Rc<Value> {
        /* Converts this macro definition to a value to be evaluated */






        todo!()
    }
}


/* Macro expander */

pub struct MacroExpander<'v> {
    evaluator: Evaluator,
    macros: HashMap<&'v String, Macro<'v>>,
    values: &'v Vec<Rc<Value>>,
}


impl<'v> MacroExpander<'v> {
    pub fn new(values: &'v Vec<Rc<Value>>) -> Self {
        /* Creates a new MacroExpander */

        MacroExpander {
            evaluator: Evaluator::new(),
            values,
            macros: HashMap::new()
        }
    }


    fn expand_macros(&mut self, value: &Rc<Value>) -> Rc<Value> {
        /* Returns value but with all macros expanded */

        match value.to_list() {
            Some(components) => {
                if components.is_empty() {
                    value.clone()
                } else if let Some(m) = self.macros.get(&components[0].name()) {
                    todo!()
                } else {
                    let new_components = components.iter()
                                            .map(|v|self.expand_macros(v))
                                            .collect();

                    Value::cons_list(&new_components)
                }
            },

            None => value.clone()
        }
    }


    pub fn macro_expand_values(&mut self) -> Result<Vec<Rc<Value>>, Error> {
        /* Returns all the values with their (self-defined) macros expanded */

        let mut macro_expanded_values = vec![];

        for value in self.values.iter() {
            match self.parse_macro_definition(value)? {
                Some(m) => {
                    self.macros.insert(m.name, m);
                },

                None => {
                    let macro_expanded = self.expand_macros(value);

                    macro_expanded_values.push(value.clone())
                }
            }
        }

        Ok(macro_expanded_values)
    }


    fn parse_macro_definition(&mut self, expr: &Rc<Value>) -> Result<Option<Macro<'v>>, Error> {
        /* Attempts to parse a macro definition */


        todo!()
    }
}


pub fn expand_macros(values: &Vec<Rc<Value>>) -> Vec<Rc<Value>> {
    /* Expands all macros in a list of exprs */

    let mut macro_expander = MacroExpander::new(values);

    macro_expander.macro_expand_values()
}
