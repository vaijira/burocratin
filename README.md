[![Pull Request](https://github.com/vaijira/burocratin/actions/workflows/main.yml/badge.svg?branch=main)](https://github.com/vaijira/burocratin/actions/workflows/main.yml)
[![Netlify Status](https://api.netlify.com/api/v1/badges/6ec7c27a-fb07-46b8-afab-07a009a40e96/deploy-status)](https://app.netlify.com/sites/upbeat-minsky-6ecee4/deploys)
# burocratin
Help with your taxes forms.

Currently supporting parsing of interactive brokers and degiro reports and generating taxes forms for D6 and aeat 720 model.

## INSTALLATION
First, you'll need Rust. To install Rust and the cargo build tool, follow the official instructions.

You also need to install the wasm32-unknown-unknown target to compile Rust to Wasm. If you're using rustup, you just need to run rustup target add wasm32-unknown-unknown.

You need to install:
* [trunk](https://trunkrs.dev/): cargo install trunk wasm-bindgen-cli.
* [cargo make](https://sagiegurari.github.io/cargo-make/): cargo install --force cargo-make.

Run cargo make, cargo make trunk and trunk serve and access http://localhost:8080 to test the application.

## TESTS
To run unit tests execute: cargo test --lib

To run doc tests execute: cargo test --doc

To run integration test you'll need docker-compose to start a selenium container:
* docker-compose up -d
* cargo test --test interaction -- --ignored --test-threads=1
* docker-compose down

