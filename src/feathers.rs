use dominator::{svg, Dom};

pub fn render_svg_help_icon(color: &str, size: &str) -> Dom {
    // <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="feather feather-help-circle"><circle cx="12" cy="12" r="10"></circle><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"></path><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>
    svg!("svg", {
        .attr("alt", "menu icon")
        .attr("width", size)
        .attr("height", size)
        .attr("viewBox", "0 0 24 24")
        .attr("fill", "none")
        .attr("stroke", color)
        .attr("stroke-width", "2")
        .attr("stroke-linecap", "round")
        .attr("stroke-linejoin", "round")
        .children(&mut[
            svg!("circle", {
                .attr("cx", "12")
                .attr("cy", "12")
                .attr("r", "10")
            }),
            svg!("path",{
                .attr("d", "M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3")
            }),
            svg!("line", {
                .attr("x1", "12")
                .attr("y1", "17")
                .attr("x2", "12.01")
                .attr("y2", "17")
            }),
        ])
    })
}
