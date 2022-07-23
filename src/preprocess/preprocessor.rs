use crate::{
    error::Error,
    eval::Evaluator,
    preprocess::{ cps, fmt, macros },
    value::Value
};

use std::rc::Rc;

pub fn preprocess(expr: &Rc<Value>, evaluator: &mut Evaluator) -> Result<Option<Rc<Value>>, Error> {
    /* Preprocesses an expression */

    if let Some(macro_expanded) = macros::expand_macros(expr, evaluator)? {
        let formatted = fmt::format_names(&macro_expanded);
        let cps_converted = cps::convert_expr(&formatted)?;

        Ok(Some(cps_converted))
    } else {
        Ok(None)
    }
}
