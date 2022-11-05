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
mod parsers;
mod reports;
mod utils;

/*
/// Main entry point for burocratin app
#[wasm_bindgen(start)]
pub fn run_app() {
    wasm_logger::init(
        wasm_logger::Config::new(log::Level::Debug).module_prefix(env!("CARGO_PKG_NAME")),
    );
    console_error_panic_hook::set_once();
    let window = web_sys::window().expect("global window does not exists");
    let document = window.document().expect("expecting a document on window");
    let element = document.get_element_by_id("burocratinApp").unwrap();
    App::<app::App>::new().mount(element);
}
*/
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
