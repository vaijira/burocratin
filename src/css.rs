use dominator::{class, pseudo};
use once_cell::sync::Lazy;

pub const DEFAULT_ICON_COLOR: &str = "black";
pub const DEFAULT_ICON_SIZE: &str = "24";

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
