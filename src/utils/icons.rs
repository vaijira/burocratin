use dominator::{svg, Dom};

pub fn render_svg_delete_square_icon(color: &str, size: &str) -> Dom {
    // <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="feather feather-x-square"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><line x1="9" y1="9" x2="15" y2="15"></line><line x1="15" y1="9" x2="9" y2="15"></line></svg>
    svg!("svg", {
        .attr("alt", "facebook icon")
        .attr("width", size)
        .attr("height", size)
        .attr("viewBox", "0 0 24 24")
        .attr("fill", "none")
        .attr("stroke", color)
        .attr("stroke-width", "2")
        .attr("stroke-linecap", "round")
        .attr("stroke-linejoin", "round")
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
