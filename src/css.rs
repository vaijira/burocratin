use dominator::{class, pseudo};
use once_cell::sync::Lazy;

pub static ROOT_CLASS: Lazy<String> = Lazy::new(|| {
    class! {
        .style("padding", "10px")
    }
});

pub static FLEX_CONTAINER_CLASS: Lazy<String> = Lazy::new(|| {
    class! {
        .style("display", "flex")
        .style("flex-flow", "wrap")
    }
});

pub static FLEX_CONTAINER_ITEM_20_CLASS: Lazy<String> = Lazy::new(|| {
    class! {
        .style("flex", "20%")
        .style("max-width", "20%")
        .style("margin-bottom", "5px")
    }
});

pub static FLEX_CONTAINER_ITEM_40_CLASS: Lazy<String> = Lazy::new(|| {
    class! {
        .style("flex", "40%")
        .style("max-width", "40%")
        .style("margin-bottom", "5px")
    }
});

pub static SECTION_HEADER: Lazy<String> = Lazy::new(|| {
    class! {
        .style("overflow", "hidden")
        .style("text-align", "center")
        .pseudo!(":after", {
            .style("background-color", "#000")
            .style("content", "''")
            .style("display", "inline-block")
            .style("height", "1px")
            .style("position", "relative")
            .style("vertical-align", "middle")
            .style("width", "50%")
            .style("right", "0.01em")
            .style("margin-right", "-50%")
        })
        .pseudo!(":before", {
            .style("background-color", "#000")
            .style("content", "''")
            .style("display", "inline-block")
            .style("height", "1px")
            .style("position", "relative")
            .style("vertical-align", "middle")
            .style("width", "50%")
            .style("right", "0.5em")
            .style("margin-left", "-50%")
        })
    }
});
