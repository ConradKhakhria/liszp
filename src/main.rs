mod read;
mod eval;
mod preprocess;

fn main() {
    /* Interface */

    let filename: String = std::env::args()
                            .filter(| s| s.ends_with(".lzp"))
                            .next()
                            .expect("liszp: no filename provided");

    let mut evaluator = eval::Evaluator::new();

    evaluator.eval_file(filename);
}
