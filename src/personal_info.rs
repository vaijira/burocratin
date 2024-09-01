use std::sync::Arc;

use dominator::{clone, events, html, with_node, Dom};
use futures_signals::signal::Mutable;
use web_sys::HtmlInputElement;

use crate::{
    css::{FLEX_CONTAINER_CLASS, FLEX_CONTAINER_ITEM_20_CLASS},
    data::{PersonalInformation, DEFAULT_YEAR},
};

pub struct PersonalInfoViewer {
    personal_info: Mutable<PersonalInformation>,
}

impl PersonalInfoViewer {
    pub fn new(personal_info: Mutable<PersonalInformation>) -> Arc<Self> {
        Arc::new(PersonalInfoViewer { personal_info })
    }

    pub fn render(this: &Arc<Self>) -> Dom {
        html!("section", {
            .class(&*FLEX_CONTAINER_CLASS)
            .child(html!("span", {
                .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
                .children(&mut [
                    html!("label", {
                        .attr("for", "name")
                        .text("Nombre: ")
                    }),
                    html!("input" => HtmlInputElement, {
                        .attr("id", "name")
                        .attr("alt", "Nombre")
                        .attr("type", "text")
                        .with_node!(element => {
                            .event(clone!(this => move |_: events::Input| {
                                this.personal_info.lock_mut().name = element.value().to_uppercase();
                            }))
                        })
                    }),
                ])
            }))
            .child(html!("span", {
                .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
                .children(&mut [
                    html!("label", {
                        .attr("for", "surname")
                        .text("Apellidos: ")
                    }),
                    html!("input" => HtmlInputElement, {
                        .attr("id", "surname")
                        .attr("alt", "Apellidos")
                        .attr("type", "text")
                        .with_node!(element => {
                            .event(clone!(this => move |_: events::Input| {
                                this.personal_info.lock_mut().surname = element.value().to_uppercase();
                            }))
                        })
                    }),
                ])
            }))
            .child(html!("span", {
                .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
                .children(&mut [
                    html!("label", {
                        .attr("for", "nif")
                        .text("NIF: ")
                    }),
                    html!("input" => HtmlInputElement, {
                        .attr("id", "nif")
                        .attr("alt", "NIF")
                        .attr("type", "text")
                        .with_node!(element => {
                            .event(clone!(this => move |_: events::Input| {
                                this.personal_info.lock_mut().nif = element.value().to_uppercase();
                            }))
                        })
                    }),
                ])
            }))
            .child(html!("span", {
                .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
                .children(&mut [
                    html!("label", {
                        .attr("for", "year")
                        .text("Año: ")
                    }),
                    html!("input" => HtmlInputElement, {
                        .attr("id", "year")
                        .attr("alt", "Año")
                        .attr("type", "text")
                        .attr("placeholder", &DEFAULT_YEAR.to_string())
                        .with_node!(element => {
                            .event(clone!(this => move |_: events::Input| {
                                this.personal_info.lock_mut().year = element.value().parse::<usize>().unwrap_or(DEFAULT_YEAR);
                            }))
                        })
                    }),
                 ])
            }))
            .child(html!("span", {
                .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
                .children(&mut [
                    html!("label", {
                        .attr("for", "phone")
                        .text("Teléfono: ")
                    }),
                    html!("input" => HtmlInputElement, {
                        .attr("id", "phone")
                        .attr("alt", "Teléfono")
                        .attr("type", "text")
                        .with_node!(element => {
                            .event(clone!(this => move |_: events::Input| {
                                this.personal_info.lock_mut().phone = element.value().to_uppercase();
                            }))
                        })
                    }),
                ])
            }))
        })
    }
}
