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


pub struct OldReader {
    filename: String,
    line_number: usize,
    column_number: usize,
}


impl OldReader {
    pub fn new(filename: String) -> Self {
        /* Creates a new Reader */

        OldReader {
            filename,
            line_number: 1,
            column_number: 1,
        }
    }


    pub fn read(&mut self, source: &String) -> Result<Vec<Rc<Value>>, Error> {
        /* Reads a source string into a list of Values */

        self.line_number = 1;
        self.column_number = 1;

        match self.read_nested_lists(source)? {
            ValueStack::List { vals, .. } => {
                Ok(vals.iter().map(Self::value_stack_to_value).collect())
            }

            ValueStack::Atom(_) => unreachable!()
        }
    }


    fn read_atom(&self, token_string: &str) -> Result<ValueStack, Error> {
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
    
            ("nil"|"null", _) => Value::Nil,
    
            _ => Value::Name(token_string.into())
        };
    
        Ok(ValueStack::Atom(value.rc()))
    }


    fn read_nested_lists(&mut self, source: &String) -> Result<ValueStack, Error> {
        /* Reads nested lists into a ValueStack */

        let mut stack = vec![ ValueStack::List { vals: vec![], delim: '?' } ];

        for token_string in Self::get_token_strings(source) {
            let first_char = match token_string.chars().next() {
                Some(c) => c,
                None => continue
            };

            match first_char {
                '#'  => {},

                ' '  => self.column_number += 1,

                '\n' => {
                    self.line_number += 1;
                    self.column_number = 1;
                }

                '('|'['|'{' => {
                    stack.push(ValueStack::List { vals: vec![], delim: first_char });
                    self.column_number += 1;
                },

                ')'|']'|'}' => {
                    self.read_closing_bracket(first_char, &mut stack)?;
                    self.column_number += 1;
                },

                _ => {
                    match stack.last_mut().expect("Liszp: unreachable error 3") {
                        ValueStack::List { vals, .. } => {
                            vals.push(self.read_atom(token_string)?);
                            self.column_number += token_string.len();
                        },

                        _ => unreachable!()
                    }
                }
            }
        }


        Ok(stack.pop().unwrap())
    }


    fn read_closing_bracket(&self, first_char: char, stack: &mut Vec<ValueStack>) -> Result<(), Error> {
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


    fn get_token_strings<'s>(source: &'s String) -> impl Iterator<Item = &'s str> {
        /* Returns an iterator of all strings found by the regex */

        lazy_static! {
            static ref REGEX: Regex = Regex::new(concat!(
                "#.*?\n|",
                r"0[bB][01_]+|0[xX][0-9a-fA-F_]+|[0-9][0-9_]*|",
                r"[a-zA-Z_\-\+\*/=<>:\.@%\&\?!][a-zA-Z0-9_\-\+\*/=<>:\.@%\&\?!]*|",
                "\".*?\"|\'.*?\'|\n|,|",
                r"\(|\)|\[|\]|\{|\}"
            )).unwrap();
        }

        REGEX.find_iter(source).map(|m| m.as_str())
    }


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


    fn value_stack_to_value(value_stack: &ValueStack) -> Rc<Value> {
        /* Recursively turns ValueStacks into Values */

        match value_stack {
            ValueStack::Atom(atom) => atom.clone(),

            ValueStack::List { vals, .. } => {
                let mut list = Value::Nil.rc();

                for v in vals.iter().rev() {
                    list = Value::cons(&Self::value_stack_to_value(v), &list).rc()
                }

                list
            }
        }
    }
}









/* New reader */

use regex::Matches;


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
                r"[a-zA-Z_\-\+\*/=<>:\.@%\&\?!][a-zA-Z0-9_\-\+\*/=<>:\.@%\&\?!]*|",
                "\".*?\"|\'.*?\'|\n|,|",
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

