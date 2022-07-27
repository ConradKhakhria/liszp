mod cps;
mod error;
mod eval;
mod macros;
mod read;
mod value;

fn main() {
    /* Interface */

    println!("All of our terrible problems could be solved with");
    println!("recursive macros. The only impediment to this is that");
    println!("the evaluator calls the macro expander, which calls");
    println!("the evaluator, etc etc. Maybe turns the macros into functions?");

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
