#![warn(
    rust_2018_idioms,
    missing_docs,
    missing_debug_implementations,
    unused_extern_crates,
    warnings
)]
#![allow(clippy::inconsistent_digit_grouping)]
//! helps with burocracy

use app::App;
use wasm_bindgen::prelude::*;

mod app;
mod css;
mod data;
mod feathers;
mod parsers;
mod reports;
mod utils;

#[wasm_bindgen(start)]
/// Main entry point for burocratin app
pub async fn main_js() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    wasm_logger::init(
        wasm_logger::Config::new(log::Level::Debug).module_prefix(env!("CARGO_PKG_NAME")),
    );

    let app = App::new();

    dominator::append_dom(&dominator::get_id("burocratinApp"), App::render(app));

    Ok(())
}
