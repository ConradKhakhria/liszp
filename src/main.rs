mod read;
mod error;
mod eval;
mod preprocess;

use std::io::Write;

fn main() {
    /* Interface */

    let mut filename = None;

    for arg in std::env::args() {
        if arg.ends_with(".lzp") {
            filename = Some(arg);
        }
    }

    let mut evaluator = eval::Evaluator::new();

    match filename {
        Some(fname) => {
            if let Err(e) = evaluator.eval_file(fname) {
                e.print();
            }
        }

        None => {
            loop {
                let input_string = repl_get_line();

                match evaluator.eval_source_string(&input_string, "<repl>") {
                    Ok(v) => println!("{}", v),
                    Err(e) => {
                        e.print();
                    }
                }
            }
        }
    }
}


fn repl_get_line() -> String {
    /* Gets a line from stdin for the REPL */

    let mut input_string = String::new();

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    print!("> ");
    stdout.flush().expect("Liszp: failed to flush stdout");

    stdin.read_line(&mut input_string).expect("Liszp: failed to read line from stdin");

    input_string
}
