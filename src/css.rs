use dominator::{class, pseudo};
use once_cell::sync::Lazy;

pub const DEFAULT_ICON_COLOR: &str = "black";
pub const DEFAULT_ICON_SIZE: &str = "32";

pub static ROOT_CLASS: Lazy<String> = Lazy::new(|| {
    class! {
        .style("padding", "10px")
    }
});

pub static ERROR_PARAGRAPH_CLASS: Lazy<String> = Lazy::new(|| {
    class! {
        .style("color", "#ba3939")
        .style("background", "#ffe0e0")
        .style("border", "1px solid #a33a3a")
        .style("text-align", "center")
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
        .style("display", "flex")
        .style("flex-direction", "row")
        .style("margin-top", "50px")
        .pseudo!(":after", {
            .style("content", "''")
            .style("flex", "1 1 66%")
            .style("border-bottom", "1px solid")
            .style("margin", "auto")
            .style("margin-left", "10px")
        })
        .pseudo!(":before", {
            .style("content", "''")
            .style("flex", "1 1 0%")
            .style("border-bottom", "1px solid")
            .style("margin", "auto")
            .style("margin-right", "10px")
        })
    }
});

pub static TOOLTIP_CONTAINER: Lazy<String> = Lazy::new(|| {
    class! {
        .style("position", "relative")
        .style("display", "inline-block")
    }
});

pub static TOOLTIP_ITEM: Lazy<String> = Lazy::new(|| {
    class! {
        .style("visibility", "hidden")
        .style("width", "300px")
        .style("background-color", "#555")
        .style("color", "#fff")
        .style("text-align", "left")
        .style("border-radius", "6px")
        .style("padding", "5px 0")
        .style("position", "absolute")
        .style("z-index", "1")
        .style("bottom", "125%")
        .style("left", "50%")
        .style("margin-left", "-60px")
        .style("opacity", "0")
        .style("transition", "opacity 0.3s")
        .pseudo!(":after", {
            .style("content", "''")
            .style("position", "absolute")
            .style("top", "100%")
            .style("left", "20%")
            .style("margin-left", "-5px")
            .style("border-width", "5px")
            .style("border-style", "solid")
            .style("border-color", "#555 transparent transparent transparent")
        })
    }
});
