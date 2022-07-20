use crate::{
    error::Error,
    new_error,
    value::Value
};

use std::rc::Rc;

use lazy_static::lazy_static;
use regex::Regex;
use rug;


// for use in the bracket collection algorithm
enum ValueStack {
    Atom(Rc<Value>),

    List {
        vals: Vec<ValueStack>,
        delim: char
    }
}


pub struct Reader {
    filename: String,
    line_number: usize,
    column_number: usize,
}


impl Reader {
    pub fn new(filename: String) -> Self {
        /* Creates a new Reader */

        Reader {
            filename,
            line_number: 1,
            column_number: 1,
        }
    }


    pub fn read(&mut self, source: &String) -> Result<Vec<Rc<Value>>, Error> {
        /* Reads a source string into a list of Values */

        self.line_number = 1;
        self.column_number = 1;

        let mut token_stream = Self::get_token_stream(source);
        let read_stream_result = self.read_stream(&mut token_stream, false);

        Ok(read_stream_result?.to_list().unwrap())
    }


    /* Helper functions */


    fn error_with_reader_position<S: ToString>(&self, msg: S) -> Error {
        /* Creates an error message with the position of the reader */

        new_error!(
            "reader error in {}:{}:{}\n{}",
            &self.filename,
            self.line_number,
            self.column_number,
            msg.to_string()
        )
    }


    fn get_token_stream<'s>(source: &'s String) -> impl Iterator<Item = &'s str> {
        /* Returns an iterator of all strings found by the regex */

        lazy_static! {
            static ref REGEX: Regex = Regex::new(concat!(
                "#.*?\n|",
                r"0[bB][01_]+|0[xX][0-9a-fA-F_]+|[0-9][0-9_]*|",
                r"[a-zA-Z_\-\+\*/=<>:\.@%\&\?!][a-zA-Z0-9_\-\+\*/=<>:\.@%\&\?!]*|",
                "\".*?\"|\'.\'|\'|\n|,|",
                r"\(|\)|\[|\]|\{|\}"
            )).unwrap();
        }

        REGEX.find_iter(source).map(|m| m.as_str())
    }


    fn read_atom(&self, token_string: &str) -> Result<Rc<Value>, Error> {
        /* Reads an atomic value */

        let value = match (token_string, token_string.chars().next().unwrap()) {
            (_, '0'..='9') => {
                let parse_int = rug::Integer::parse(token_string);
                let parse_float = rug::Float::parse(token_string);

                if let Ok(i) = parse_int {
                    Value::Integer(rug::Integer::from(i))
                } else if let Ok(f) = parse_float {
                    Value::Float(rug::Float::with_val(53, f))
                } else {
                    return self.error_with_reader_position(format!("Could not parse '{}' as a number", token_string)).into()
                }
            },

            ("true", _) => Value::Bool(true),

            ("false", _) => Value::Bool(false),
    
            (_, '"') => Value::String(token_string.into()),

            (c, '\'') => Value::String(String::new() + c),
    
            ("nil"|"null", _) => Value::Nil,
    
            _ => Value::Name(token_string.into())
        };
    
        Ok(value.rc())
    }


    fn read_stream<'s>(&mut self, stream: &mut impl Iterator<Item = &'s str>, recursive_call: bool) -> Result<Rc<Value>, Error> {
       /* Reads a token stream
        *
        * parameter 'recursive_call' says whether the call to read_stream()
        * was made recursively.
        */

        let mut elements = vec![];

        while let Some(token_string) = stream.next() {
            let first_char = match token_string.chars().next() {
                Some(c) => c,
                None => {
                    eprintln!("Found a zero-width token string");
                    continue;
                }
            };

            match first_char {
                '#'  => continue,

                ' '  => self.column_number += 1,

                '\n' => {
                    self.line_number += 1;
                    self.column_number = 1;
                }

                '('|'['|'{' => {
                    self.column_number += 1;
                    elements.push(self.read_stream(stream, true)?);
                },

                ')'|']'|'}' => {
                    self.column_number += 1;

                    if recursive_call {
                        break;
                    } else {
                        unimplemented!();
                    }
                },

                _ => {
                    self.column_number += token_string.len();
                    elements.push(self.read_atom(token_string)?);
                }
            }
        }

        Ok(Value::cons_list(&elements))
    }


    /* Helper functions */






    fn old_read_closing_bracket(&self, first_char: char, stack: &mut Vec<ValueStack>) -> Result<(), Error> {
        /* Reads a closing bracket */

        let (list_vals, list_delim) = match stack.pop().expect("Liszp: unreachable error 1") {
            ValueStack::List { vals, delim } => (vals, delim),
            ValueStack::Atom(_) => unreachable!()
        };

        let expected = match list_delim {
            '(' => ')',
            '[' => ']',
             _ => '}'
        };

        if first_char != expected {
            if stack.len() == 0 {
                return self.error_with_reader_position(format!("unexpected closing bracket '{}'", first_char)).into();
            } else {
                return self.error_with_reader_position(format!(
                    "Liszp: expected expr opened with '{}' to be closed with '{}', found '{}' instead",
                    list_delim, expected, first_char
                )).into();
            }
        }

        match stack.last_mut().expect("Liszp: unreachable error 2") {
            ValueStack::List { vals, .. } => {
                vals.push(ValueStack::List { vals: list_vals, delim: list_delim });
            },

            _ => unreachable!()
        }

        Ok(())
    }

}
