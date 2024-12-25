# Burocratin

[![Pull Request](https://github.com/vaijira/burocratin/actions/workflows/main.yml/badge.svg?branch=main)](https://github.com/vaijira/burocratin/actions/workflows/main.yml)
![GitHub Sponsors](https://img.shields.io/github/sponsors/vaijira?logo=Github&label=Sponsor&color=fe8e86)

It helps with your taxes forms.

Currently supporting parsing of interactive brokers and degiro reports and generating taxes forms for D6 and aeat 720 model.

## INSTALLATION

First, you'll need Rust. To install Rust and the cargo build tool, follow the official instructions.

You also need to install the wasm32-unknown-unknown target to compile Rust to Wasm. If you're using rustup, you just need to run rustup target add wasm32-unknown-unknown.

You need to install:

* yarn: npm install yarn.

Run yarn install and yarn start and access [localhost](http://localhost:10001) to test the application.

## TESTS

To run unit tests execute: cargo test --lib

To run doc tests execute: cargo test --doc

To run integration test you'll need docker-compose to start a selenium container:

* yarn install
* yarn run build
* docker-compose up -d
* cargo test --test interaction -- --ignored --test-threads=1
* docker-compose down
