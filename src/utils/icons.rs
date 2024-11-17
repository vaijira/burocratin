use dominator::{svg, Dom, DomBuilder};
use web_sys::SvgElement;

fn svg_icon_attrs(icon: DomBuilder<SvgElement>) -> DomBuilder<SvgElement> {
    icon.attr("viewBox", "0 0 24 24")
        .attr("fill", "none")
        .attr("stroke-width", "2")
        .attr("stroke-linecap", "round")
        .attr("stroke-linejoin", "round")
}

pub fn render_svg_trash_icon(color: &str, size: &str) -> Dom {
    svg!("svg", {
        .attr("alt", "trash icon")
        .attr("width", size)
        .attr("height", size)
        .attr("stroke", color)
        .apply(svg_icon_attrs)
        .children(&mut[
            svg!("polyline", {
                .attr("points", "3 6 5 6 21 6")
            }),
            svg!("path", {
                .attr("d", "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2")
            }),
            svg!("line", {
                .attr("x1", "10")
                .attr("y1", "11")
                .attr("x2", "10")
                .attr("y2", "17")
            }),
            svg!("line", {
                .attr("x1", "14")
                .attr("y1", "11")
                .attr("x2", "14")
                .attr("y2", "17")
            }),
        ])
    })
}

pub fn render_svg_save_icon(color: &str, size: &str) -> Dom {
    svg!("svg", {
        .attr("alt", "save icon")
        .attr("width", size)
        .attr("height", size)
        .attr("stroke", color)
        .apply(svg_icon_attrs)
        .children(&mut[
            svg!("path", {
                .attr("d", "M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z")
            }),
            svg!("polyline", {
                .attr("points", "17 21 17 13 7 13 7 21")
            }),
            svg!("polyline", {
                .attr("points", "7 3 7 8 15 8")
            }),
        ])
    })
}

pub fn render_svg_edit_icon(color: &str, size: &str) -> Dom {
    svg!("svg", {
        .attr("alt", "edit icon")
        .attr("width", size)
        .attr("height", size)
        .attr("stroke", color)
        .apply(svg_icon_attrs)
        .children(&mut[
            svg!("path", {
                .attr("d", "M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7")
            }),
            svg!("path", {
                .attr("d", "M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z")
            }),
        ])
    })
}

pub fn render_svg_cancel_icon(color: &str, size: &str) -> Dom {
    svg!("svg", {
        .attr("alt", "cancel icon")
        .attr("width", size)
        .attr("height", size)
        .attr("stroke", color)
        .apply(svg_icon_attrs)
        .children(&mut[
            svg!("rect", {
                .attr("x", "3")
                .attr("y", "3")
                .attr("width", "18")
                .attr("height", "18")
                .attr("rx", "2")
                .attr("ry", "2")
            }),
            svg!("line", {
                .attr("x1", "9")
                .attr("y1", "9")
                .attr("x2", "15")
                .attr("y2", "15")
            }),
            svg!("line", {
                .attr("x1", "15")
                .attr("y1", "9")
                .attr("x2", "9")
                .attr("y2", "15")
            }),
        ])

    })
}
