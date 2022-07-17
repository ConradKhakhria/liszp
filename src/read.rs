use crate::{
    error::Error,
    new_error,
    value::Value
};

use std::collections::LinkedList;
use std::rc::Rc;

use lazy_static::lazy_static;
use regex::Regex;
use rug;



#[derive(Debug)]
enum ValueStack {
    Atom(Rc<Value>),

    List {
        vals: LinkedList<ValueStack>,
        delim: char
    }
}

fn read_atom(string: String) -> Result<ValueStack, Error> {
    /* Convets the source string of an atomic value into a Value */

    let value = match (&string[..], string.chars().next().unwrap()) {
        (_, '0'..='9') => {
            let parse_int = rug::Integer::parse(&string);
            let parse_flt = rug::Float::parse(&string);

            match (parse_int, parse_flt) {
                (Ok(i), _) => Value::Integer(rug::Integer::from(i)),
                (_, Ok(f)) => Value::Float(rug::Float::with_val(53, f)),
                _ => return new_error!("could not parse '{}' as an integer or a float", string).into()
            }
        },

        ("true"|"false", _) => Value::Bool(&string[..] == "true"),

        (_, '"') => Value::String(string),

        ("nil"|"null", _) => Value::Nil,

        _ => Value::Name(string)
    };

    Ok(ValueStack::Atom(value.rc()))
}


fn read_nested_lists(source: &String, filename: String) -> Result<ValueStack, Error> {
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

    macro_rules! error_with_reader_position {
        ($msg:literal, $( $binding:expr ),*) => {
            new_error!("syntax error in {}:{}:{}\n{}", filename, line_number, column_number, format!($msg, $($binding,)*))
        };
    }

    lazy_static! {
        static ref REGEX: Regex = Regex::new(concat!(
            "#.*?\n|",
            r"0[bB][01_]+|0[xX][0-9a-fA-F_]+|[0-9][0-9_]*|",
            r"[a-zA-Z_\-\+\*/=<>:\.@%\&\?!][a-zA-Z0-9_\-\+\*/=<>:\.@%\&\?!]*|",
            "\".*?\"|\'.*?\'|\n|,|",
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
            '#' => {},

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
                let (lvals, ldelim) = match stack.pop_back().expect("Liszp: unreachable error 1") {
                    ValueStack::List { vals, delim } => (vals, delim),
                    ValueStack::Atom(_) => unreachable!()
                };

                let expected = match ldelim {
                    '(' => ')',
                    '[' => ']',
                     _ => '}'
                };

                if first_char != expected {
                    if stack.len() == 0 {
                        error_with_reader_position!("unexpected closing bracket '{}'", first_char);
                    } else {
                        error_with_reader_position!(
                            "Liszp: expected expr opened with '{}' to be closed with '{}', found '{}' instead",
                            ldelim, expected, first_char
                        );
                    }
                }

                match stack.back_mut().expect("Liszp: unreachable error 2") {
                    ValueStack::List { vals, .. } => {
                        vals.push_back(ValueStack::List { vals: lvals, delim: ldelim });
                    },
                    _ => unreachable!()
                }

                column_number += 1;
            },

            _ => {
                let atom = read_atom(expr_str.into())?;

                match stack.back_mut().expect("Liszp: unreachable error 3") {
                    ValueStack::List { vals, .. } => {
                        vals.push_back(atom);
                        column_number += expr_str.len();
                    },
                    _ => unreachable!()
                };
            }
        }
    }

    Ok(stack.pop_front().unwrap())
}


pub fn read(source: &String, filename: String) -> Result<Vec<Rc<Value>>, Error> {
   /* Reads a source string into an array of Values */

    fn rec_read(stack: &ValueStack) -> Rc<Value> {
        /* Recursively turns ValueStacks into Values (including LinkedList -> Value::Cons) */

        match stack {
            ValueStack::Atom(atom) => atom.clone(),
            ValueStack::List { vals, .. } => {
                let mut list = Value::Nil.rc();

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

    let values = match read_nested_lists(source, filename)? {
        ValueStack::List { vals, .. } => vals,
        _ => unreachable!()
    };

    Ok(values.iter().map(rec_read).collect())
}
