//! The headless check: `cargo run -p moonroad --bin moonroad-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<moonroad::MoonRoad>("moonroad")
}
