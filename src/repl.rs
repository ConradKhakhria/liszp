use crate::error::Error;
use crate::eval::Evaluator;
use crate::new_error;
use crate::read;
use crate::value::Value;
use std::io::Write;
use std::rc::Rc;


fn get_line_from_stdin(display_prompt: bool) -> Result<String, Error> {
    /* Reads a line from stdin */

    let mut input_string = String::new();

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    if display_prompt {
        print!("> ");
    } else {
        print!("  ");
    }

    if let Err(_) = stdout.flush() {
        return new_error!("failed to flush stdout").into();
    }

    if let Err(_) = stdin.read_line(&mut input_string) {
        return new_error!("failed to read line from stdin").into();
    }

    Ok(input_string)
}


fn brackets_are_balanced(string: &String) -> Result<bool, Error> {
    /* Returns whether a string has balanced brackets */

    let mut bracket_depth = 0;

    for c in string.chars() {
        match c {
            '('|'['|'{' => bracket_depth += 1,
            ')'|']'|'}' => bracket_depth -= 1,
            _ => {}
        }
    }

    if bracket_depth < 0 {
        new_error!("input string has more closing braces than opening braces").into()
    } else {
        Ok(bracket_depth == 0)
    }
}


fn repl_iteration(evaluator: &mut Evaluator) -> Result<Rc<Value>, Error> {
    /* Performs one iteration of the repl */

    let mut input_string = get_line_from_stdin(true)?;

    while !brackets_are_balanced(&input_string)? {
        input_string = format!("{}{}", input_string, get_line_from_stdin(false)?);
    }

    if input_string == "exit" {
        panic!("cya");
    }

    let exprs = read::read(&input_string, &"<repl>".to_string(), false)?;

    if exprs.len() == 1 {
        evaluator.eval(&exprs[0])
    } else {
        new_error!("REPL can only evaluate 1 expression at a time").into()
    }
}


pub fn run_repl() {
    /* runs a REPL until an exit is reached */

    let mut evaluator = Evaluator::new();

    if let Err(e) = evaluator.load_stdlib() {
        eprintln!("{}", e.display(false));
    }

    loop {
        match repl_iteration(&mut evaluator) {
            Ok(v) => println!("{}", v),
            Err(e) => eprintln!("{}", e.display(false))
        }
    }
}


