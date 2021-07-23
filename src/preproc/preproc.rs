use crate::read::Value;
use crate::preproc::{ cps, fmt };

use std::rc::Rc;

pub fn preprocess(expr: Rc<Value>) -> Rc<Value> {
    /* Preprocesses a value */

    let formatted = fmt::format_names(expr);
    let if_rearranged = cps::move_ifs(formatted);
    let cps_converted = cps::convert(if_rearranged, None);

    return cps_converted;
}
