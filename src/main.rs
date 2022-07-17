mod error;
mod eval;
mod read;
mod preprocess;
mod value;

fn main() {
    /* Interface */

    let mut filename = None;

    for arg in std::env::args() {
        if arg.ends_with(".lzp") {
            if let Some(_) = filename {
                panic!("Liszp: you must provide at most one file");
            } else {
                filename = Some(arg);
            }
        }
    }

    let mut evaluator = eval::Evaluator::new();

    match filename {
        Some(fname) => {
            if let Err(e) = evaluator.eval_file(fname) {
                e.println();
            }
        }

        None => loop {
            match evaluator.repl_iteration() {
                Ok(value) => println!("{}", value),
                Err(e) => e.println()
            }
        }
    }
}
