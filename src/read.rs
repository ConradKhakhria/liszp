use crate::{
    error::Error,
    new_error,
    refcount_list,
    value::Value
};

use std::rc::Rc;

use lazy_static::lazy_static;
use regex::{ Matches, Regex };
use rug;


pub struct Reader<'s> {
    column: usize,
    line: usize,
    filename: String,
    open_bracket_strings: Vec<&'s str>,
    token_stream: Matches<'static, 's>,
}


type ReaderResult = Result<Option<Rc<Value>>, Error>;


impl<'s> Reader<'s> {
    pub fn new(source: &'s String, filename: &String) -> Self {
        /* Creates a new Reader */

        lazy_static! {
            static ref REGEX: Regex = Regex::new(concat!(
                "#.*?\n|",
                r"0[bB][01_]+|0[xX][0-9a-fA-F_]+|[0-9][0-9_]*|",
                r"[a-zA-Z_\-\+\*/=<>:\.@%\?!][a-zA-Z0-9_\-\+\*/=<>:\.@%\&\?!]*|",
                "\".*?\"|\'.\'|\'|\n|`|,|",
                r"\(|\)|\[|\]|\{|\}"
            )).unwrap();
        }

        Reader {
            column: 1,
            line: 1,
            filename: filename.clone(),
            open_bracket_strings: vec![],
            token_stream: REGEX.find_iter(source)
        }
    }


    fn error_with_reader_position<S: ToString>(&self, msg: S) -> Error {
        /* Creates an error message with the position of the reader */

        new_error!(
            "reader error in {}:{}:{}\n{}",
            &self.filename,
            self.line,
            self.column,
            msg.to_string()
        )
    }


    pub fn read(&mut self) -> ReaderResult {
        /* Reads one value from the stream */

        if let Some(token_match) = self.token_stream.next() {
            match token_match.as_str() {
                b @ ("("|"["|"{") => self.read_list(b),

                b @ (")"|"]"|"}") => self.match_closing_bracket(b),

                "'" => {
                    match self.read()? {
                        Some(v) => {
                            let wrapped_expr = refcount_list![ Value::Name("quote".into()).rc(), v ];

                            Ok(Some(wrapped_expr))
                        },

                        None => Ok(None)
                    }
                }

                "`" => {
                    match self.read()? {
                        Some(v) => {
                            let wrapped_expr = refcount_list![ Value::Name("quasiquote".into()).rc(), v ];

                            Ok(Some(wrapped_expr))
                        },

                        None => Ok(None)
                    }
                }

                "," => {
                    match self.read()? {
                        Some(v) => {
                            let wrapped_expr = refcount_list![ Value::Name("unquote".into()).rc(), v ];

                            Ok(Some(wrapped_expr))
                        },

                        None => Ok(None)
                    }
                }

                atom => self.read_atom(atom)
            }
        } else {
            match self.open_bracket_strings.pop() {
                None => Ok(None),
                _ => self.error_with_reader_position("unexpected EOF (unclosed brackets)").into()
            }
        }
    }


    pub fn read_atom(&mut self, atom: &'s str) -> ReaderResult {
        /* Reads an atomic expression */

        self.column += atom.len();

        let value = match (atom, atom.chars().next().unwrap()) {
            ("\n", _)|(_, '#')=> {
                self.column = 1;
                self.line += 1;

                return self.read();
            },

            (_, '0'..='9') => {
                let parse_int = rug::Integer::parse(atom);
                let parse_float = rug::Float::parse(atom);

                if let Ok(i) = parse_int {
                    Value::Integer(rug::Integer::from(i))
                } else if let Ok(f) = parse_float {
                    Value::Float(rug::Float::with_val(53, f))
                } else {
                    return self.error_with_reader_position(format!("Could not parse '{}' as a number", atom)).into()
                }
            },

            ("true", _) => Value::Bool(true),

            ("false", _) => Value::Bool(false),
    
            (_, '"') => Value::String(atom.into()),

            (_, '\'') => Value::String(format!("{}", atom)),
    
            ("nil"|"null", _) => Value::Nil,
    
            _ => Value::Name(atom.into())
        };
    
        Ok(Some(value.rc()))
    }


    pub fn read_list(&mut self, opening_bracket: &'s str) -> ReaderResult {
        /* Reads a list expression */

        self.column += opening_bracket.len();
        self.open_bracket_strings.push(opening_bracket);

        let mut list_elements = vec![];

        while let Some(elem) = self.read()? {
            list_elements.push(elem);
        }

        Ok(Some(Value::cons_list(&list_elements)))
    }


    pub fn match_closing_bracket(&mut self, closing_bracket: &'s str) -> ReaderResult {
        /* Matches the closing bracket of a list with the expected */

        self.column += closing_bracket.len();

        let expected_opening_bracket = match closing_bracket {
            ")" => "(",
            "]" => "[",
            "}" => "{",
             _  => unreachable!()
        };

        match self.open_bracket_strings.pop() {
            Some(opening_bracket) => {
                if opening_bracket == expected_opening_bracket {
                    Ok(None)
                } else {
                    let msg = format!("Closing bracket '{}' does not match '{}'", closing_bracket, opening_bracket);

                    self.error_with_reader_position(msg).into()
                }
            },

            None => {
                let msg = format!("Unexpected closing bracket '{}'", closing_bracket);

                self.error_with_reader_position(msg).into()
            }
        }
    }
}


pub fn read(source: &String, filename: &String) -> Result<Vec<Rc<Value>>, Error> {
    /* Reads a source string into a vec of values */

    let mut reader = Reader::new(source, filename);
    let mut values = vec![];

    while let Some(value) = reader.read()? {
        values.push(value);
    }

    Ok(values)
}

