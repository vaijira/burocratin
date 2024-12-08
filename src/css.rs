use dominator::{class, pseudo};
use once_cell::sync::Lazy;

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
        .style("gap", "5px")
    }
});

pub static FLEX_CONTAINER_ITEM_20_CLASS: Lazy<String> = Lazy::new(|| {
    class! {
        .style("flex", "auto")
        .style("margin", "5px")
        .style("font-size", "small")
        // .style("max-width", "20%")
    }
});

pub static FLEX_CONTAINER_ITEM_40_CLASS: Lazy<String> = Lazy::new(|| {
    class! {
        .style("flex", "40%")
        .style("max-width", "40%")
        .style("margin", "5px")
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

pub static TABLE_STYLE: Lazy<String> = Lazy::new(|| {
    class! {
        .style("overflow", "auto")
        .style("width", "100%")
        .style("height", "400px")
        .style("border-collapse", "collapse")
        .style("border", "1px solid #8c8c8c")
        .style("margin-bottom" ,"1em")
    }
});

pub static TABLE_CAPTION: Lazy<String> = Lazy::new(|| {
    class! {
        .style("font-size", "large")
        .style("margin", "20px")
    }
});

pub static TABLE_HEADER: Lazy<String> = Lazy::new(|| {
    class! {
        .style("font-size", "small")
    }
});

pub static TABLE_ROW: Lazy<String> = Lazy::new(|| {
    class! {
        .style("font-size", "small")
        .pseudo!(":nth-child(even)", {
            .style("background-color", "#f2f2f2")
        })
    }
});
