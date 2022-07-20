use crate::{
    error::Error,
    new_error,
    value::Value
};

use std::rc::Rc;

use lazy_static::lazy_static;
use regex::{ Matches, Regex };
use rug;


pub struct Reader<'s> {
    token_stream: Matches<'static, 's>,
    filename: String,
    line_number: usize,
    column_number: usize,
}


impl<'s> Reader<'s> {
    pub fn new(source: &'s String, filename: String) -> Self {
        /* Creates a new Reader */

        lazy_static! {
            static ref REGEX: Regex = Regex::new(concat!(
                "#.*?\n|",
                r"0[bB][01_]+|0[xX][0-9a-fA-F_]+|[0-9][0-9_]*|",
                r"[a-zA-Z_\-\+\*/=<>:\.@%\&\?!][a-zA-Z0-9_\-\+\*/=<>:\.@%\&\?!]*|",
                "\".*?\"|\'.\'|\'|\n|,|",
                r"\(|\)|\[|\]|\{|\}"
            )).unwrap();
        }

        Reader {
            token_stream: REGEX.find_iter(source),
            filename,
            line_number: 1,
            column_number: 1,
        }
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


    fn match_closing_bracket(&self, opening_char: char, closing_bracket: char) -> Result<(), Error> {
        /* Reads the closing bracket of an expression */

        let expected = match opening_char {
            '(' => ')',
            '[' => ']',
             _ => '}'
        };

        if closing_bracket != expected {
            self.error_with_reader_position(format!(
                "Liszp: expected expr opened with '{}' to be closed with '{}', found '{}' instead",
                opening_char, expected, closing_bracket
            )).into()
        } else {
            Ok(())
        }
    }


    fn read_stream(&mut self, opening_char: char, recursive_call: bool) -> Result<Rc<Value>, Error> {
       /* Reads a token stream
        *
        * parameter 'recursive_call' says whether the call to read_stream()
        * was made recursively.
        */

        let mut elements = vec![];

        while let Some(token_match) = self.token_stream.next() {
            let token_string = token_match.as_str();
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
                    elements.push(self.read_stream(first_char, true)?);
                },

                ')'|']'|'}' => {
                    self.column_number += 1;

                    if recursive_call {
                        self.match_closing_bracket(opening_char, first_char)?;

                        return Ok(Value::cons_list(&elements));
                    } else {
                        let msg = format!("found unmatched closing '{}'", first_char);

                        return self.error_with_reader_position(msg).into()
                    }
                },

                _ => {
                    self.column_number += token_string.len();
                    elements.push(self.read_atom(token_string)?);
                }
            }
        }

        if recursive_call {
            self.error_with_reader_position("missing closing bracket or brackets").into()
        } else {
            Ok(Value::cons_list(&elements))
        }
    }
}


pub fn read<S: ToString>(source: &String, filename: S) -> Result<Vec<Rc<Value>>, Error> {
    /* Reads a source string into a list of values */

    let mut reader = Reader::new(source, filename.to_string());

    reader
        .read_stream('?', false)
        .map(|v| v.to_list().unwrap())  
}
