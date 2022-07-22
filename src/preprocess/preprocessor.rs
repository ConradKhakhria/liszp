use crate::{
    error::Error,
    preprocess::{ cps, fmt, macros },
    value::Value
};

use std::rc::Rc;


pub struct Preprocessor {
    macro_expander: macros::MacroExpander,
    preprocessed: Vec<Rc<Value>>
}


impl Preprocessor {
    pub fn new() -> Self {
        /* Creates a new Preprocessor */

        Preprocessor {
            macro_expander: macros::MacroExpander::new(),
            preprocessed: vec![]
        }
    }


    pub fn preprocess_program(&mut self, exprs: &Vec<Rc<Value>>) -> Result<Vec<Rc<Value>>, Error> {
        /* Preprocesses the constituent expressions of a program */

        let mut preprocessed = vec![];

        for expr in exprs.iter() {
            if let Some(v) = self.preprocess(expr)? {
                preprocessed.push(v);
            }
        }

        Ok(preprocessed)
    }


    pub fn preprocess(&mut self, expr: &Rc<Value>) -> Result<Option<Rc<Value>>, Error> {
        /* Preprocesses an expression */

        if let Some(macro_expanded) = self.macro_expander.expand_macros(expr)? {
            let formatted = fmt::format_names(&macro_expanded);
            let cps_converted = cps::convert_expr(&formatted)?;

            self.preprocessed.push(cps_converted.clone());

            Ok(Some(cps_converted))
        } else {
            Ok(None)
        }
    }
}
