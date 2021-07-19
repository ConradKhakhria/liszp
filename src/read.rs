use std::collections::LinkedList;
use std::rc::Rc;

use lazy_static::lazy_static;
use regex::Regex;
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
    pub fn eq(self: &Rc<Value>, other: &Rc<Value>) -> bool {
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
            panic!("Attempt to get length of something that isn't a list");
        }
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
                format!("({})", Value::print_list(Rc::new(self.clone())))
            },
            Value::Quote(xs) => {
                format!("'({})", Value::print_list(Rc::clone(xs)))
            },
            Value::Nil => {
                "nil".into()
            }
        });
    }
}

#[derive(Debug)]
enum ValueStack {
    Atom(Rc<Value>),

    List {
        vals: LinkedList<ValueStack>,
        delim: char
    }
}

fn read_atom(string: String) -> ValueStack {
    /* Convets the source string of an atomic value into a Value */

    let value = match (&string[..], string.chars().next().unwrap()) {
        (_, '0'..='9') => {
            let parse_int = string.parse::<i64>();
            let parse_flt = string.parse::<i64>();

            match (parse_int, parse_flt) {
                (Ok(i), _) => Value::Integer(rug::Integer::from(i)),
                (_, Ok(f)) => Value::Float(rug::Float::with_val(53, f)),
                _ => panic!("Liszp: could not parse '{}' as an integer or a float", string)
            }
        },

        ("true"|"false", _) => Value::Bool(&string[..] == "true"),

        (_, '"') => Value::String(string),

        ("nil"|"null", _) => Value::Nil,

        _ => Value::Name(string)
    };

    return ValueStack::Atom(Rc::new(value));
}

fn read_nested_lists(source: &String, filename: String) -> ValueStack {
   /* O(n) nested list parser
    *
    * This function converts a source string into a 'ValueStack', which is
    * either an atomic Value or a LinkedList of ValueStacks. After the entire
    * source string has been read, the ValueStack can be converted into Values.
    */

    let base_value = ValueStack::List { vals: LinkedList::new(), delim: '?' };
    let mut stack = LinkedList::new();

    stack.push_back(base_value);

    let mut line_number = 0;
    let mut column_number = 0;

    let rnl_err = |i| format!("Liszp: internal error in read_nested_lists() :: {}", i);

    lazy_static! {
        static ref REGEX: Regex = Regex::new(concat!(
            "#.*?\n|",
            r"0[bB][01_]+|0[xX][0-9a-fA-F_]+|[0-9][0-9_]*|",
            r"[a-zA-Z_\-\+\*/=<>:\.@%\&\?][a-zA-Z0-9_\-\+\*/=<>:\.@%\&\?]*|",
            "\".*?\"|\'.*?\'|\n|,|!|",
            r"\(|\)|\[|\]|\{|\}"
        )).unwrap();
    }

    for ex in REGEX.find_iter(&source) {
        let expr_str = ex.as_str();

        if expr_str == "" {
            continue;
        }

        let first_char = expr_str.chars().next().unwrap();

        match first_char {
            ' '  => column_number += 1,
            '\n' => {
                column_number = 1;
                line_number  += 1;
            },

            '('|'{'|'[' => {
                let new_block = ValueStack::List {
                    vals: LinkedList::new(),
                    delim: first_char
                };

                stack.push_back(new_block);
                column_number += 1;
            },

            ')'|'}'|']' => {
                let (lvals, ldelim) = match stack.pop_back().expect(&rnl_err(1)[..]) {
                    ValueStack::List { vals, delim } => (vals, delim),
                    ValueStack::Atom(_) => panic!("{}", rnl_err(3))
                };

                let expected = match ldelim {
                    '(' => ')',
                    '[' => ']',
                     _ => '}'
                };

                if first_char != expected {
                    panic!(
                        "Liszp: expected expr opened with '{}' to be closed with '{}', found '{}' instead",
                        ldelim, expected, first_char
                    );
                }

                if let ValueStack::List { vals, .. } = stack.back_mut().expect(&rnl_err(2)[..]) {
                    vals.push_back(ValueStack::List { vals: lvals, delim: ldelim });
                } else {
                    panic!("{}", rnl_err(4));
                }

                column_number += 1;
            },

            _ => {
                let atom = read_atom(expr_str.into());

                match stack.back_mut().expect(&rnl_err(5)[..]) {
                    ValueStack::List { vals, .. } => {
                        vals.push_back(atom);
                        column_number += expr_str.len();
                    },
                    _ => panic!("{}", rnl_err(6))
                };
            }
        }
    }

    return stack.pop_front().unwrap();
}

pub fn read(source: &String, filename: String) -> LinkedList<Rc<Value>> {
   /* Reads a source string into a linked list of Values */

    fn rec_read(stack: &ValueStack) -> Rc<Value> {
        /* Recursively turns ValueStacks into Values (including LinkedList -> Value::Cons) */

        match stack {
            ValueStack::Atom(atom) => Rc::clone(atom),
            ValueStack::List { vals, .. } => {
                let mut list = Rc::new(Value::Nil);

                for val in vals.iter().rev() {
                    list = Rc::new(Value::Cons {
                        car: rec_read(val),
                        cdr: list
                    });
                }

                return list;
            }
        }
    }

    let values = match read_nested_lists(source, filename) {
        ValueStack::List { vals, .. } => vals,
        _ => panic!("Liszp: internal error in function read() :: 1")
    };

    return values.iter().map(rec_read).collect();
}
