use crate::{
    error::Error,
    preprocess::{ cps, fmt },
    value::Value
};

use std::rc::Rc;


pub fn preprocess(expr: Rc<Value>) -> Result<Rc<Value>, Error> {
    /* Preprocesses a value */

    let formatted = fmt::format_names(&expr);
    let cps_converted = cps::convert_expr(&formatted)?;

    Ok(cps_converted)
}
