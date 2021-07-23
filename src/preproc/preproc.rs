use crate::read::Value;
use crate::preproc::{ cps, fmt };

use std::rc::Rc;

pub fn preprocess(value: Rc<Value>) -> Rc<Value> {
    /* Preprocesses a value */

    let formatted = fmt::format_names(value);
    let cps_converted = cps::convert(formatted, None, None);

    return cps_converted;
}
