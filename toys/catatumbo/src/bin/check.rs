//! The headless check: `cargo run -p catatumbo --bin catatumbo-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<catatumbo::Catatumbo>("catatumbo")
}
