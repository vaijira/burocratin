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

pub fn render_svg_twitter_icon(color: &str, size: &str) -> Dom {
    // <svg alt="X twitter icon" width="24" height="24" viewBox="0 0 24 24"><path d="m 18.744792,0 h 3.850107 l -8.453496,10.079827 9.876362,13.673506 H 16.267332 L 10.199228,15.444049 3.2522952,23.753333 H -0.59781209 L 8.3578721,12.9723 -1.1,0 h 7.9429388 l 5.4822182,7.5905485 z m -1.347537,21.386764 h 2.134298 L 5.7213859,2.2789175 H 3.4280611 Z" style="stroke-width:0.0856513"></path></svg>
    svg!("svg", {
        .attr("alt", "X twitter icon")
        .attr("width", size)
        .attr("height", size)
        .attr("viewBox", "0 0 24 24")
        .attr("fill", color)
        .attr("stroke", color)
        .attr("stroke-width", "0.0856513")
        .attr("stroke-linecap", "round")
        .attr("stroke-linejoin", "round")
        .children(&mut[
            svg!("path",{
                .attr("d", "m 18.744792,0 h 3.850107 l -8.453496,10.079827 9.876362,13.673506 H 16.267332 L 10.199228,15.444049 3.2522952,23.753333 H -0.59781209 L 8.3578721,12.9723 -1.1,0 h 7.9429388 l 5.4822182,7.5905485 z m -1.347537,21.386764 h 2.134298 L 5.7213859,2.2789175 H 3.4280611 Z")
            }),
        ])
    })
}

pub fn render_svg_facebook_icon(color: &str, size: &str) -> Dom {
    // <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="feather feather-facebook"><path d="M18 2h-3a5 5 0 0 0-5 5v3H7v4h3v8h4v-8h3l1-4h-4V7a1 1 0 0 1 1-1h3z"></path></svg>
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
            svg!("path",{
                .attr("d", "M18 2h-3a5 5 0 0 0-5 5v3H7v4h3v8h4v-8h3l1-4h-4V7a1 1 0 0 1 1-1h3z")
            }),
        ])
    })
}

pub fn render_svg_instagram_icon(color: &str, size: &str) -> Dom {
    // <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="feather feather-instagram"><rect x="2" y="2" width="20" height="20" rx="5" ry="5"></rect><path d="M16 11.37A4 4 0 1 1 12.63 8 4 4 0 0 1 16 11.37z"></path><line x1="17.5" y1="6.5" x2="17.51" y2="6.5"></line></svg>
    svg!("svg", {
        .attr("alt", "instagram icon")
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
                .attr("x", "2")
                .attr("y", "2")
                .attr("width", "20")
                .attr("height", "20")
                .attr("rx", "5")
                .attr("ry", "5")
            }),
            svg!("path",{
                .attr("d", "M16 11.37A4 4 0 1 1 12.63 8 4 4 0 0 1 16 11.37z")
            }),
            svg!("line", {
                .attr("x1", "17.5")
                .attr("y1", "6.5")
                .attr("x2", "17.51")
                .attr("y2", "6.5")
            }),
        ])
    })
}

pub fn render_svg_linkedin_icon(color: &str, size: &str) -> Dom {
    // <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="feather feather-linkedin"><path d="M16 8a6 6 0 0 1 6 6v7h-4v-7a2 2 0 0 0-2-2 2 2 0 0 0-2 2v7h-4v-7a6 6 0 0 1 6-6z"></path><rect x="2" y="9" width="4" height="12"></rect><circle cx="4" cy="4" r="2"></circle></svg>
    svg!("svg", {
        .attr("alt", "linkedin icon")
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
                .attr("x", "2")
                .attr("y", "9")
                .attr("width", "4")
                .attr("height", "12")
            }),
            svg!("path",{
                .attr("d", "M16 8a6 6 0 0 1 6 6v7h-4v-7a2 2 0 0 0-2-2 2 2 0 0 0-2 2v7h-4v-7a6 6 0 0 1 6-6z")
            }),
            svg!("circle", {
                .attr("cx", "4")
                .attr("cy", "4")
                .attr("r", "2")
            }),
        ])
    })
}
