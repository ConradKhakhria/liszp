/* The code in this file transforms a source string into a list of Exprs */

use std::collections::LinkedList;
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone, Debug)]
pub enum Expr {
    Name { // e.g. a variable name
        string: String,
        position: (usize, usize)
    },

    StringLiteral {
        string: String,
        position: (usize, usize)
    },

    NumberLiteral {
        string: String,
        position: (usize, usize)
    },

    List {
        body: LinkedList<Expr>,
        delim: String,
        position: (usize, usize)
    },
}

#[allow(dead_code)]
impl Expr {
    pub fn equals(&self, s: String) -> bool {
        return match self {
            Expr::Name          { string, .. } => s == *string,
            Expr::StringLiteral { string, .. } => s == *string,
            Expr::NumberLiteral { string, .. } => s == *string,
            _ => false
        };
    }

    pub fn has_delim(&self, s: String) -> bool {
        return match self {
            Expr::List { delim, .. } => *delim == s,
            _ => false
        };
    }

    pub fn has_type(&self, t: String) -> bool {
        return match self {
            Expr::Name          {..} => t == "Name",
            Expr::StringLiteral {..} => t == "StringLiteral",
            Expr::NumberLiteral {..} => t == "NumberLiteral",
            Expr::List          {..} => t == "List",
        };
    }

    pub fn atomic(&self) -> bool {
        return match self {
            Expr::List {..} => false,
            _ => true
        };
    }

    pub fn position(&self) -> (usize, usize) {
        return match *self {
            Expr::Name          { position, .. } => position,
            Expr::StringLiteral { position, .. } => position,
            Expr::NumberLiteral { position, .. } => position,
            Expr::List          { position, .. } => position
        };
    }

    pub fn inner_values(&self) -> LinkedList<Expr> {
        /* Returns the inner Exprs of a list */

        return match self {
            Expr::List { body, .. } => body.clone(),
            _ => LinkedList::new()
        };
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Expr::List { body, delim, .. } => {
                let mut string = format!("{}", delim);

                for ex in body.iter() {
                    string = format!("{}{}", string, ex);
                }

                string.push_str(match &delim[..] {
                    "(" => ")",
                    "[" => "]",
                     _  => "}"
                });

                write!(f, "{}", string)
            },

            Expr::Name          { string, .. } => write!(f, " {} ", string),
            Expr::NumberLiteral { string, .. } => write!(f, " {} ", string),
            Expr::StringLiteral { string, .. } => write!(f, " {} ", string)
        };
    }
}

pub fn tokenise(source: &String, pos: (usize, usize)) -> LinkedList<Expr> {
   /* Takes a source string and returns a list of tokens
    * 
    * args
    * ----
    * - source: the raw source string of the module
    * - pos: the initial position of the source string
    *
    * returns
    * -------
    * A Result of a vector of expressions or a vector of errors
    * 
    * note
    * ----
    * all names have "&" appended to them so that automatically generated
    * names (during CPS conversion and macro expansion) aren't shadowed
    */

    let mut lineno: usize = pos.0;
    let mut colno:  usize = pos.1;

    let base_expr = Expr::List { body: LinkedList::new(), delim: "".into(), position: pos };
    let mut expr_stack = vec![ base_expr ];

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
        let exprstr: &str = ex.as_str();

        if exprstr.chars().next().unwrap() == '#' {
            lineno += 1;
            colno   = 1;
            continue;
        }

        match exprstr {
            ""   => {},

            " "  => colno += 1,

            "\n" => {
                lineno += 1;
                colno   = 1;
            },

            "("|"["|"{" => {
                let new_block = Expr::List {
                    body: LinkedList::new(),
                    delim: exprstr.into(),
                    position: (lineno, colno)
                };

                expr_stack.push(new_block);
                colno += exprstr.len();
            },

            ")"|"]"|"}" => {
                let top_index = expr_stack.len() - 1;
                let expected;

                match expr_stack.last().unwrap() {
                    Expr::List { delim, ..} => {
                        expected = match delim.as_str() {
                           "(" => ")",
                           "[" => "]",
                           "{" => "}",
                            _  => panic!("Unexpected closing brace '{}' at {}:{}", delim, lineno, colno)
                        };
                    },

                    _ => panic!("liszp: Internal error in function tokenise() :: 2")
                };

                if exprstr != expected {
                    panic!("Expected closing bracket '{}', received '{}'", expected, exprstr);
                }

                let final_block = expr_stack.pop().unwrap();

                match &mut expr_stack[top_index - 1] {
                    Expr::List { body, .. } => body.push_back(final_block),
                    _ => panic!("ccdm: Internal error in function tokenise() :: 3")
                };

                colno += exprstr.len();
            },

            _ => {
                let top_index = expr_stack.len() - 1;

                match &mut expr_stack[top_index] {
                    Expr::List { body, .. } => {
                        body.push_back(match exprstr.chars().nth(0).unwrap() {
                            '0'..='9' => Expr::NumberLiteral {
                                string: exprstr.into(),
                                position: (lineno, colno)
                            },

                            '"'|'\'' => Expr::StringLiteral {
                                string: exprstr.into(),
                                position: (lineno, colno)
                            },

                            _ => Expr::Name {
                                string: exprstr.into(),
                                position: (lineno, colno)
                            }
                        });
                    },

                    _ => panic!("ccdm: Internal error in function tokenise() :: 4")
                };

                colno += exprstr.len();
            }
        }
    }

    return match &expr_stack[0] {
        Expr::List { body, .. } => body.clone(),
        _ => panic!("ccdm: Internal error in function tokenise() :: 5")
    };
}
