//! The headless check: `cargo run -p biobay --bin biobay-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<biobay::MosquitoBay>("biobay")
}
