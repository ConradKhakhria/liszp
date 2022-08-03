mod error;
mod eval;
mod macros;
mod read;
mod repl;
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

    match filename {
        Some(fname) => {
            let mut evaluator = eval::Evaluator::new();

            if let Err(e) = evaluator.load_stdlib() {
                e.println();
                panic!("error in stdlib");
            }

            if let Err(e) = evaluator.eval_file(fname) {
                e.println();
            }
        }

        None => repl::run_repl()
    }
}
