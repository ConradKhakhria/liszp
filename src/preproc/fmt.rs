use crate::read::Value;
use std::rc::Rc;


pub (in crate::preproc) fn format_names(value: Rc<Value>) -> Rc<Value> {
    /* Appends an ampersand to each name to avoid collision with 'no-continuation' */

    return match &*value {
        Value::Name(name) => {
            Value::Name(format!("&{}", name)).rc()
        },

        Value::Cons { car, cdr } => {
            Rc::new(Value::Cons {
                car: format_names(Rc::clone(car)),
                cdr: format_names(Rc::clone(cdr))
            })
        },

        _ => value
    }
}
