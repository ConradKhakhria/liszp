use crate::lexer::Expr;

use std::collections::LinkedList;
use std::rc::Rc;
use rug;

#[derive(Clone, Debug)]
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
    fn eq(self: &Rc<Value>, other: &Rc<Value>) -> bool {
        return match (&**self, &**other) {
            (Value::Name(a), Value::Name(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Cons { car: a, cdr: x}, Value::Cons { car: b, cdr: y }) => {
                a.eq(&b) && x.eq(&y)
            },
            (Value::Quote(xs), Value::Quote(ys)) => xs.eq(&ys),
            (Value::Nil, Value::Nil) => true,
            _ => false
        };
    }

    pub fn len(&self) -> i64 {
        /* Gets the length of a cons list */

        let mut cursor = &Rc::new(self.clone());
        let mut length = -1;

        while let Value::Cons { cdr, .. } = &**cursor {
            cursor  = &cdr; // ew
            length += 1
        }

        if length > -1 {
            return length + 1;
        } else {
            println!("{:?}", self);
            panic!("Attempt to get length of something that isn't a list");
        }
    }

    pub fn index(&self, index: usize) -> Rc<Value> {
        /* Indexes a cons list */

        let mut value  = Rc::new(self.clone());
        let mut cursor = Rc::clone(&value);
 
        for _ in 0..index+1 {
            if let Value::Cons { car, cdr} = &*cursor {
                value  = Rc::clone(car);
                cursor = Rc::clone(cdr);
            } else {
                panic!("Liszp internal error: index out of bounds");
            }
        }

        return Rc::clone(&value);
    }

    pub fn is_cons(&self) -> bool {
        /* is &self a cons? */

        return match self {
            Value::Cons {..} => true,
            _ => false
        };
    }

    pub fn is_nil(&self) -> bool {
        /* Is &self a Value::Nil */

        return match self {
            Value::Nil => true,
            _ => false
        };
    }

    pub fn to_list(&self) -> Option<LinkedList<Rc<Value>>> {
        /* Converts a cons list to a std::collections::LinkedList<SharedVal> */

        if let Value::Nil = self {
            return Some(LinkedList::new());
        }

        let mut cursor = self;
        let mut count = 0;
        let mut list = LinkedList::new();

        while let Value::Cons { car, cdr } = cursor {
            list.push_back(Rc::clone(car));
            cursor = &cdr;

            count += 1;
        }

        return if count == 0 {
            None
        } else {
            Some(list)
        };
    }

    pub fn name(&self) -> String {
        /* if self = Value::Name(n) then n else String::new() */

        return match self {
            Value::Name(n) => n.clone(),
            _ => String::new()
        };
    }

    pub fn refcounted(&self) -> Rc<Value> {
        /* Value -> Rc<Value> */
    
        return Rc::new(self.clone());
    }
}

#[macro_export]
macro_rules! value_list {
    [ $( $x:expr ),* ] => {
        {
            let mut cons = Rc::new(Value::Nil);
            let mut expr_list = LinkedList::new();

            $(
                expr_list.push_front($x);
            )*

            for ex in expr_list.into_iter() {
                cons = Rc::new(Value::Cons {
                    car: Rc::new(ex.clone()),
                    cdr: cons
                });
            }

            cons
        }
    };
}

#[macro_export]
macro_rules! refcount_list {
    [ $( $x:expr ),* ] => {
        {
            let mut cons = Rc::new(Value::Nil);
            let mut expr_list = LinkedList::new();

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

impl<'a> std::fmt::Display for Value {
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
                format!("\"{}\"", s)
            },
            Value::Bool(b) => {
                format!("{}", b)
            },
            Value::Cons { .. } => {
                format!("({})", print_list(Rc::new(self.clone())))
            },
            Value::Quote(xs) => {
                format!("'({})", print_list(Rc::clone(xs)))
            },
            Value::Nil => {
                "nil".into()
            }
        });
    }
}

fn print_list<'a>(xs: Rc<Value>) -> String {
    let mut string = String::new();
    let mut cursor = &xs;

    while let Value::Cons { car, cdr } = &**cursor {
        string += &format!(" {}", *car)[..];
        cursor  = &cdr;
    }

    match **cursor {
        Value::Nil => {},
        _ => string += &format!(" . {}", cursor)
    }

    return (&string[1..]).into();
}

pub fn parse<'a>(expr: &'a Expr) -> Rc<Value> {
   /* Parses an expression (string or list of strings) into a value
    *
    * args
    * ----
    * - expr: the expression to be parsed
    *
    * returns
    * -------
    * The value received. Panics if there's an error
    */

    match expr {
        Expr::Name { string, .. } => {
            return match &string[..] {
                "nil"   => Value::Nil.refcounted(),
                "true"  => Value::Bool(true).refcounted(),
                "false" => Value::Bool(false).refcounted(),
                _       => Value::Name(string.clone()).refcounted()
            };
        },

        Expr::StringLiteral { string, .. } => return Value::String(string.clone()).refcounted(),

        Expr::NumberLiteral { string, position } => {
            match string.find('.') {
                Some(_) => {
                    let res: f64 = string.parse::<f64>()
                                    .expect(&format!("Failed to parse '{}' as float ({:?})", string, position)[..]);
        
                    return Value::Float(rug::Float::with_val(53, res)).refcounted()
                },
        
                None => {
                    let res: i64 = string.parse::<i64>()
                                    .expect(&format!("Failed to parse '{}' as int ({:?})", string, position)[..]);
        
                    return Value::Integer(rug::Integer::from(res)).refcounted()
                }
            };
        },

        Expr::List { body, position: _, .. } => {
            let mut value = Value::Nil.refcounted();

            for expr in body.iter().rev() {
                value = Rc::new(Value::Cons {
                    car: parse(expr),
                    cdr: value
                });
            }

            return value;
        }
    }
}
