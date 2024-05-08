use crate::feathers::{
    render_svg_facebook_icon, render_svg_instagram_icon, render_svg_linkedin_icon,
    render_svg_twitter_icon,
};
use dominator::{html, Dom};

pub(crate) fn render_footer() -> Dom {
    html!("footer", {
    .text("Comparte en tus redes sociales si te ha sido de utilidad.")
    .children(&mut[
        html!("div",{
            .children(&mut[
                html!("span", {
                    .style("margin", "2px")
                    .child(
                        html!("a", {
                            .attr("alt", "Compartir en twitter")
                            .attr("aria-label", "Compartir en twitter")
                            .attr("href", "https://twitter.com/intent/tweet?text=Te ayuda a rellenar el modelo 720&url=https://www.burocratin.com")
                            .attr("target", "_blank")
                            .attr("rel", "external nofollow")
                            .child(render_svg_twitter_icon("black", "24"))
                        })
                    )
                }),
                html!("span", {
                    .style("margin", "5px")
                    .child(
                        html!("a", {
                            .attr("alt", "Compartir en facebook")
                            .attr("aria-label", "Compartir en facebook")
                            .attr("href", "https://www.facebook.com/sharer/sharer.php?u=www.burocratin.com")
                            .attr("target", "_blank")
                            .attr("rel", "external nofollow")
                            .child(render_svg_facebook_icon("blue", "24"))
                        })
                    )
                }),
                html!("span", {
                    .style("margin", "5px")
                    .child(
                        html!("a", {
                            .attr("alt", "Compartir en instagram")
                            .attr("aria-label", "Compartir en instagram")
                            .attr("href", "https://www.instagram.com")
                            .attr("target", "_blank")
                            .attr("rel", "external nofollow")
                            .child(render_svg_instagram_icon("darkviolet", "24"))
                        })
                    )
                }),
                html!("span", {
                    .style("margin", "5px")
                    .child(
                        html!("a", {
                            .attr("alt", "Compartir en linkedin")
                            .attr("aria-label", "Compartir en linkedin")
                            .attr("href", "https://www.linkedin.com/sharing/share-offsite/?url=https://www.burocratin.com")
                            .attr("target", "_blank")
                            .attr("rel", "external nofollow")
                            .child(render_svg_linkedin_icon("blue", "24"))
                        })
                    )
                }),

            ])
        }),
        html!("p", {
            .text("Para cualquier mejora, duda, sugerencia o error puedes crear un ")
            .child(
                html!("a", {
                    .attr("href", "https://github.com/vaijira/burocratin/issues/new?title=Error&body=He%20encontrado%20un%20error")
                    .attr("alt", "informar de problemas o sugerencias")
                    .attr("target", "_blank")
                    .attr("rel", "external nofollow")
                    .text("ticket")
                })
            )
            .text(".")
        }),
        ]
    )})
}
