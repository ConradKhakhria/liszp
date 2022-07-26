use std::rc::Rc;


#[macro_export]
macro_rules! refcount_list {
    [ $( $x:expr ),* ] => {
        {
            let mut cons = Value::Nil.rc();
            let mut expr_list = std::collections::LinkedList::new();

            $(
                expr_list.push_front($x);
            )*

            for ex in expr_list.iter() {
                cons = Rc::new(Value::Cons {
                    car: Rc::clone(ex),
                    cdr: cons
                });
            }

            cons
        }
    };
}


#[derive(Debug)]
pub enum Value {
    Name(String),

    Integer(rug::Integer),

    Float(rug::Float),

    String(String),

    Bool(bool),

    Cons {
        car: Rc<Value>,
        cdr: Rc<Value>
    },

    Quote(Rc<Value>), // Value::Cons

    Nil
}


impl Value {

    /* Methods */


    pub fn name(&self) -> String {
        /* if self = Value::Name(n) then n else String::new() */

        return match self {
            Value::Name(n) => n.clone(),
            _ => String::new()
        };
    }


    pub fn rc(self) -> Rc<Value> {
        /* Value -> Rc<Value> */
    
        return Rc::new(self);
    }


    pub fn to_list(&self) -> Option<Vec<Rc<Value>>> {
        /* Converts a cons list to a Vec<Rc<Value>> */

        if let Value::Nil = self {
            return Some(vec![]);
        }

        let mut cursor = self;
        let mut count = 0;
        let mut list = vec![];

        while let Value::Cons { car, cdr } = cursor {
            list.push(car.clone());
            cursor = &cdr;

            count += 1;
        }

        return if count == 0 {
            None
        } else {
            Some(list)
        };
    }


    /* Namespaced functions */

    pub fn cons(car: &Rc<Value>, cdr: &Rc<Value>) -> Value {
        /* Creates a cons pair */

        Value::Cons {
            car: car.clone(),
            cdr: cdr.clone()
        }
    }


    pub fn cons_list(xs: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Creates a cons list out of an iterable */

        let mut cursor = Value::Nil.rc();

        for x in xs.into_iter().rev() {
            cursor = Value::cons(x, &cursor).rc();
        }

        cursor
    }


    fn print_list(xs: &Value) -> String {
        let mut string = String::new();
        let mut cursor = xs;

        while let Value::Cons { car, cdr } = cursor {
            string += &format!(" {}", *car)[..];
            cursor  = &cdr;
        }

        match cursor {
            Value::Nil => {},
            _ => string += &format!(" . {}", cursor)
        }

        string
    }


    pub fn substitute(expr: &Rc<Value>, old: &Rc<Value>, new: &Rc<Value>) -> Rc<Value> {
        /* Returns self but with replacing a certain expression */

        if std::ptr::eq(&**expr, &**old) {
            new.clone()
        } else if let Value::Cons { car, cdr} = &**expr {
            Rc::new(Value::Cons {
                car: Self::substitute(car, old, new),
                cdr: Self::substitute(cdr, old, new)
            })
        } else {
            expr.clone()
        }
    }
}


impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", match self {
            Value::Name(s) => {
                format!("{}", s)
            },
            Value::Integer(i) => {
                format!("{}", i)
            },
            Value::Float(f) => {
                format!("{}", f)
            },
            Value::String(s) => {
                format!("{}", s)
            },
            Value::Bool(b) => {
                format!("{}", b)
            },
            Value::Cons { .. } => {
                format!("({})", Value::print_list(self))
            },
            Value::Quote(v) => {
                format!("'{}", v)
            },
            Value::Nil => {
                "nil".into()
            }
        });
    }
}


impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Name(a), Value::Name(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Cons { car: a, cdr: x}, Value::Cons { car: b, cdr: y }) => {
                a == b && x == y
            },
            (Value::Quote(x), Value::Quote(y)) => x == y,
            (Value::Nil, Value::Nil) => true,
            _ => false
        }
    }
}


impl<T> Into<Result<Value, T>> for Value {
    fn into(self) -> Result<Value, T> {
        /* Wraps self in a result */

        Ok(self)
    }
}