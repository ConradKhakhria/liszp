mod read;
mod eval;
mod preprocess;

fn main() {
    /* Interface */

    let filename: String = std::env::args()
                            .filter(| s| s.ends_with(".lzp"))
                            .next()
                            .expect("liszp: no filename provided");

    let display_evaluated = std::env::args().filter(|s| &s[..] == "--vals").next().is_some();
    let display_namespace = std::env::args().filter(|s| &s[..] == "--ns").next().is_some();

    let mut evaluator = eval::Evaluator::new();

    evaluator.eval_file(filename);

    if display_evaluated {
        evaluator.display_evaluated();
    }

    if display_namespace {
        evaluator.display_namespace();
    }
}
