use crate::value::Value;
use std::rc::Rc;


pub fn format_names(value: &Rc<Value>) -> Rc<Value> {
    /* Appends an ampersand to each name to avoid collision with 'no-continuation' */

    return match &**value {
        Value::Name(name) => {
            match name.chars().next() {
                Some('&') => value.clone(),
                Some(_)   => Value::Name(format!("&{}", name)).rc(),
                None      => value.clone()
            }
        },

        Value::Cons { car, cdr } => {
            Rc::new(Value::Cons {
                car: format_names(car),
                cdr: format_names(cdr)
            })
        },

        _ => value.clone()
    }
}
