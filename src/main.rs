mod read;
mod eval;
mod preprocess;

use std::rc::Rc;

fn main() {
    /* Interface */

    let filename: String = std::env::args()
                            .filter(| s| s.ends_with(".lzp"))
                            .next()
                            .expect("liszp: no filename provided");

    let display_evaluated = std::env::args().filter(|s| &s[..] == "--vals").next().is_some();
    let display_namespace = std::env::args().filter(|s| &s[..] == "--ns").next().is_some();

    /* Read */

    let source = std::fs::read_to_string(filename.clone()).unwrap();
    let exprs: Vec<Rc<read::Value>> = read::read(&source, filename)
                                                .iter()
                                                .map(|v| preprocess::preprocess(Rc::clone(v)))
                                                .collect();

    /* eval */

    let mut results = Vec::new();
    let mut env = eval::Env::new();

    for value in exprs.iter() {
        results.push(env.eval(value));
    }

    if display_evaluated {
        println!("\n:: values ::\n");

        for (i, r) in results.iter().enumerate() {
            println!("expr {} evaluates to {};", i + 1, *r);
        }
    }

    if display_namespace {
        println!("\n:: global namespace ::\n");

        let globals = env.get_globals();

        for k in globals.keys() {
            println!("value '{}' = {}", k, globals.get(k).unwrap());
        }
    }
}
