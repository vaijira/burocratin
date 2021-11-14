#![warn(
    rust_2018_idioms,
    missing_docs,
    missing_debug_implementations,
    unused_extern_crates,
    warnings
)]

//! helps with burocracy

#![recursion_limit = "4096"]
use wasm_bindgen::prelude::*;
use yew::prelude::*;

mod account_notes;
mod app;
mod d6_filler;
mod degiro_parser;
mod pdf_parser;
mod reports;

/// Main entry point for burocratin app
#[wasm_bindgen(start)]
pub fn run_app() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    App::<app::App>::new().mount_to_body();
}
