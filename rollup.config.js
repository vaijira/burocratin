import rust from "@wasm-tool/rollup-plugin-rust";
import serve from "rollup-plugin-serve";
import livereload from "rollup-plugin-livereload";
import { terser } from "rollup-plugin-terser";
import copy from 'rollup-plugin-copy';

const is_watch = !!process.env.ROLLUP_WATCH;

export default {
    input: {
        index: "./Cargo.toml",
    },
    output: {
        dir: "dist/js",
        format: "iife",
        sourcemap: true,
    },
    plugins: [
        rust({
            serverPath: "js/",
        }),

        copy({
            targets: [
                { src: 'static/css/main.css', dest: 'dist/css/' },
                { src: 'static/img/degiro.svg', dest: 'dist/img/' },
                { src: 'static/img/interactive_brokers.svg', dest: 'dist/img/' },
                { src: 'static/img/favicon.ico', dest: 'dist/img/' },
                { src: 'static/img/burocratin.svg', dest: 'dist/img/' },
            ]
        }),

        is_watch && serve({
            contentBase: "dist",
            open: true,
        }),

        is_watch && livereload("dist"),

        !is_watch && terser(),
    ],
};
