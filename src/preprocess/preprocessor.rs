use crate::{
    error::Error,
    eval::Evaluator,
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


    pub fn preprocess(&mut self, expr: &Rc<Value>, evaluator: &mut Evaluator) -> Result<Option<Rc<Value>>, Error> {
        /* Preprocesses an expression */

        if let Some(macro_expanded) = self.macro_expander.expand_macros(expr, evaluator)? {
            let formatted = fmt::format_names(&macro_expanded);
            let cps_converted = cps::convert_expr(&formatted)?;

            self.preprocessed.push(cps_converted.clone());

            Ok(Some(cps_converted))
        } else {
            Ok(None)
        }
    }
}
