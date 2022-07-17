use crate::value::Value;
use crate::preprocess::{ cps, fmt };
use std::rc::Rc;


pub fn preprocess(expr: Rc<Value>) -> Rc<Value> {
    /* Preprocesses a value */

    let formatted = fmt::format_names(&expr);
    let cps_converted = cps::convert_expr(&formatted);

    cps_converted
}
