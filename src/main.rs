mod lexer;
mod preproc;
mod parse;
mod eval;

use std::collections::HashMap;

fn main() {
    /* Interface */

    let filename: String = std::env::args()
                            .filter(| s| s.ends_with(".lzp"))
                            .next()
                            .expect("liszp: no filename provided");

    let display_evaluated = std::env::args().filter(|s| &s[..] == "--vals").next().is_some();
    let display_namespace = std::env::args().filter(|s| &s[..] == "--ns").next().is_some();

    /* Read */

    let source = std::fs::read_to_string(filename).unwrap();
    let exprs = lexer::tokenise(&source, (1, 1));

    /* eval */

    let mut results = Vec::new();
    let mut globals = HashMap::new();

    for expr in exprs.iter() {
        results.push(eval::eval(parse::parse(expr), &mut globals));
    }

    if display_evaluated {
        println!("\n:: values ::\n");

        for (i, r) in results.iter().enumerate() {
            println!("expr {} evaluates to {};", i + 1, *r);
        }
    }

    if display_namespace {
        println!("\n:: global namespace ::\n");

        for k in globals.keys() {
            println!("value '{}' = {}", k, globals.get(k).unwrap());
        }
    }
}
