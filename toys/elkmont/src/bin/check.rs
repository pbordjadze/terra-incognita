//! The headless check: `cargo run -p elkmont --bin elkmont-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<elkmont::Elkmont>("elkmont")
}
