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
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    match filename {
        Some(fname) => evaluator.eval_file(fname),

        None => loop {
            let mut input_string = String::new();

            print!("> ");
            stdout.flush().expect("some error message");

            stdin.read_line(&mut input_string).expect("Liszp: failed to read line from stdin");

            match read::read(&input_string, "<repl>".into()).map(|v| v.as_slice()) {
                Ok([expr]) => {
                    let preprocessed = preprocess::preprocess(expr.clone());
                    println!("{}", evaluator.eval(&preprocessed));
                },

                Err(e) => {
                    e.print();
                }

                _ => panic!("Liszp: repl can only read one expr at a time")
            }
        }
    }
}
