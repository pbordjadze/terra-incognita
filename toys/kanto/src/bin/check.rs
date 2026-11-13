//! The headless check: `cargo run -p kanto --bin kanto-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<kanto::Kanto>("kanto")
}
