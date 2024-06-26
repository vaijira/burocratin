use std::sync::Arc;

use dominator::{clone, events, html, stylesheet, with_node, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use gloo_file::{futures::read_as_bytes, Blob};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

use crate::css::{
    ERROR_PARAGRAPH_CLASS, FLEX_CONTAINER_CLASS, FLEX_CONTAINER_ITEM_20_CLASS,
    FLEX_CONTAINER_ITEM_40_CLASS, ROOT_CLASS, SECTION_HEADER,
};
use crate::data::{AccountNote, BalanceNote, BrokerInformation, FinancialInformation};
use crate::parsers::degiro_csv::DegiroCSVParser;
use crate::parsers::ib::IBParser;
use crate::parsers::ib_csv::IBCSVParser;
use crate::parsers::{degiro::DegiroParser, pdf::read_pdf};
use crate::tooltip::Tooltip;
use crate::utils::web;

const DEFAULT_YEAR: usize = 2023;

pub struct App {
    current_error: Mutable<Option<String>>,
    degiro_broker: Arc<BrokerInformation>,
    ib_broker: Arc<BrokerInformation>,
    account_notes: MutableVec<AccountNote>,
    balance_notes: MutableVec<BalanceNote>,
    aeat720_form_path: Mutable<Option<String>>,
    name: Mutable<String>,
    surname: Mutable<String>,
    nif: Mutable<String>,
    year: Mutable<usize>,
    phone: Mutable<String>,
    degiro_pdf_tooltip: Arc<Tooltip>,
    degiro_csv_tooltip: Arc<Tooltip>,
    ib_tooltip: Arc<Tooltip>,
    ib_csv_tooltip: Arc<Tooltip>,
}

impl App {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            current_error: Mutable::new(None),
            degiro_broker: Arc::new(BrokerInformation::new(
                String::from("Degiro"),
                String::from("NL"),
            )),
            ib_broker: Arc::new(BrokerInformation::new(
                String::from("Interactive Brokers"),
                String::from("IE"),
            )),
            account_notes: MutableVec::new(),
            balance_notes: MutableVec::new(),
            aeat720_form_path: Mutable::new(None),
            name: Mutable::new("".to_owned()),
            surname: Mutable::new("".to_owned()),
            nif: Mutable::new("".to_owned()),
            year: Mutable::new(DEFAULT_YEAR),
            phone: Mutable::new("".to_owned()),
            degiro_pdf_tooltip: Tooltip::new(),
            degiro_csv_tooltip: Tooltip::new(),
            ib_tooltip: Tooltip::new(),
            ib_csv_tooltip: Tooltip::new(),
        })
    }

    fn generate_720_file(app: Arc<Self>) {
        let old_path = (*app.aeat720_form_path.lock_ref()).clone();
        let old_path = old_path.map_or("".to_owned(), |x| x);
        if let Ok(path) = web::generate_720(
            &FinancialInformation {
                account_notes: app.account_notes.lock_ref().to_vec(),
                balance_notes: app.balance_notes.lock_ref().to_vec(),
                name: app.name.lock_ref().clone(),
                surname: app.surname.lock_ref().clone(),
                nif: app.nif.lock_ref().clone(),
                year: *app.year.lock_ref(),
                phone: app.phone.lock_ref().clone(),
            },
            &old_path,
        ) {
            *app.aeat720_form_path.lock_mut() = Some(path);
        }
    }

    fn read_degiro_pdf(app: Arc<Self>, content: Vec<u8>) {
        if let Ok(data) = read_pdf(&content) {
            let parser = DegiroParser::new(data, &app.degiro_broker);
            let pdf_content = parser.parse_pdf_content();
            if let Ok((balance_notes, account_notes)) = pdf_content {
                app.account_notes
                    .lock_mut()
                    .retain(|note| note.broker != app.degiro_broker);
                app.balance_notes
                    .lock_mut()
                    .retain(|note| note.broker != app.degiro_broker);
                app.account_notes.lock_mut().extend(account_notes);
                app.balance_notes.lock_mut().extend(balance_notes);
                *app.current_error.lock_mut() = None;
            } else {
                *app.current_error.lock_mut() = Some(format!(
                    "Error cargando los apuntes del pdf de Degiro: {}",
                    pdf_content.err().unwrap()
                ));
            }
        } else {
            *app.current_error.lock_mut() = Some("Error parseando el pdf de Degiro".to_string());
        }

        App::generate_720_file(app)
    }

    fn read_degiro_csv(app: Arc<Self>, content: Vec<u8>) {
        if let Ok(data) = String::from_utf8(content) {
            let parser = DegiroCSVParser::new(data, &app.degiro_broker);
            let csv_content = parser.parse_csv();
            if let Ok(balance_notes) = csv_content {
                app.account_notes
                    .lock_mut()
                    .retain(|note| note.broker != app.degiro_broker);
                app.balance_notes
                    .lock_mut()
                    .retain(|note| note.broker != app.degiro_broker);
                app.balance_notes.lock_mut().extend(balance_notes);
                *app.current_error.lock_mut() = None;
            } else {
                *app.current_error.lock_mut() = Some(format!(
                    "Error cargando los apuntes del csv de Degiro: {}",
                    csv_content.err().unwrap()
                ));
            }
        } else {
            *app.current_error.lock_mut() = Some("Error parseando el csv de Degiro".to_string());
        }

        App::generate_720_file(app)
    }

    fn read_ib_csv(app: Arc<Self>, content: Vec<u8>) {
        if let Ok(data) = String::from_utf8(content) {
            if let Ok(parser) = IBCSVParser::new(data, &app.ib_broker) {
                let account_notes = parser.parse_account_notes();
                let balance_notes = parser.parse_balance_notes();
                if let (Ok(account_notes), Ok(balance_notes)) = (account_notes, balance_notes) {
                    app.account_notes
                        .lock_mut()
                        .retain(|note| note.broker != app.ib_broker);
                    app.balance_notes
                        .lock_mut()
                        .retain(|note| note.broker != app.ib_broker);
                    app.account_notes.lock_mut().extend(account_notes);
                    app.balance_notes.lock_mut().extend(balance_notes);
                    *app.current_error.lock_mut() = None;
                } else {
                    *app.current_error.lock_mut() = Some(
                        "Error cargando los apuntes del csv de interactive brokers".to_string(),
                    );
                }
            } else {
                *app.current_error.lock_mut() =
                    Some("Error leyendo compañías del csv de interactive brokers".to_string());
            }
        } else {
            *app.current_error.lock_mut() =
                Some("Error parseando el csv de interactive brokes".to_string());
        }

        App::generate_720_file(app)
    }

    fn read_ib_html(app: Arc<Self>, content: Vec<u8>) {
        if let Ok(data) = String::from_utf8(content) {
            if let Ok(parser) = IBParser::new(&data, &app.ib_broker) {
                let account_notes = parser.parse_account_notes();
                let balance_notes = parser.parse_balance_notes();
                if let (Ok(account_notes), Ok(balance_notes)) = (account_notes, balance_notes) {
                    app.account_notes
                        .lock_mut()
                        .retain(|note| note.broker != app.ib_broker);
                    app.balance_notes
                        .lock_mut()
                        .retain(|note| note.broker != app.ib_broker);
                    app.account_notes.lock_mut().extend(account_notes);
                    app.balance_notes.lock_mut().extend(balance_notes);
                    *app.current_error.lock_mut() = None;
                } else {
                    *app.current_error.lock_mut() = Some(
                        "Error cargando los apuntes del html de interactive brokers".to_string(),
                    );
                }
            }
        } else {
            *app.current_error.lock_mut() =
                Some("Error parseando el html de interactive brokes".to_string());
        }

        App::generate_720_file(app)
    }

    fn render_degiro_pdf_input(app: Arc<Self>) -> Dom {
        html!("span", {
            .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
            .child(Tooltip::render(app.degiro_pdf_tooltip.clone(),
                html!("p", {
                    .text(" Para descargar las posiciones de degiro en PDF: ")
                }),
                html!("ul", {
                    .children(&mut [
                        html!("li", {
                            .text("Entre en la página de degiro con su usuario.")
                        }),
                        html!("li", {
                            .text("En el menú izquierdo pulse Actividad y seguidamente informes.")
                        }),
                        html!("li", {
                            .text("En informes seleccione el informe anual del año a declarar y pulse descargar.")
                        }),
                    ])
                }))
            )
            .child(
                html!("input" => HtmlInputElement, {
                    .attr("id", "degiro_pdf_report")
                    .attr("alt", "Fichero PDF informe broker Degiro")
                    .attr("accept", "application/pdf")
                    .attr("type", "file")
                    .with_node!(element => {
                        .event(clone!(app => move |_: events::Change| {
                            let file_list = match element.files() {
                                Some(file_list) => file_list,
                                None => {
                                    *app.current_error.lock_mut() = Some(
                                    "Error subiendo fichero pdf degiro".to_string());
                                    return;
                                }
                            };
                            let degiro_pdf_data = match file_list.get(0) {
                                Some(data) => data,
                                None => {
                                    *app.current_error.lock_mut() = Some(
                                    "Error obteniendo pdf degiro".to_string());
                                    return;
                                }
                            };
                            let blob = Blob::from(degiro_pdf_data);
                            spawn_local(clone!(app => async move {
                                App::read_degiro_pdf(app, read_as_bytes(&blob).await.unwrap());
                            }));
                        }))
                    })
                })
            )
        })
    }

    fn render_degiro_csv_input(app: Arc<Self>) -> Dom {
        html!("span", {
            .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
            .child(Tooltip::render(app.degiro_csv_tooltip.clone(),
                html!("p", {
                    .text(" Para descargar las posiciones de degiro en CSV: ")
                }),
                html!("ul", {
                    .children(&mut [
                        html!("li", {
                            .text("Entre en la página de degiro con su usuario.")
                        }),
                        html!("li", {
                            .text("En el menú izquierdo pulse Cartera.")
                        }),
                        html!("li", {
                            .text("Arriba a la derecha pulse el botón exportar.")
                        }),
                        html!("li", {
                            .text("Seleccione la fecha de 31 de Diciembre del año a declarar y pulse CSV.")
                        }),
                    ])
                }))
            )
            .child(
                html!("input" => HtmlInputElement, {
                    .attr("id", "degiro_csv_report")
                    .attr("alt", "Fichero CSV informe broker Degiro")
                    .attr("accept", "text/csv")
                    .attr("type", "file")
                    .with_node!(element => {
                        .event(clone!(app => move |_: events::Change| {
                            let file_list = match element.files() {
                                Some(file_list) => file_list,
                                None => {
                                    *app.current_error.lock_mut() = Some(
                                    "Error subiendo fichero csv degiro".to_string());
                                    return;
                                }
                            };
                            let degiro_csv_data = match file_list.get(0) {
                                Some(data) => data,
                                None => {
                                    *app.current_error.lock_mut() = Some(
                                    "Error obteniendo csv degiro".to_string());
                                    return;
                                }
                            };
                            let blob = Blob::from(degiro_csv_data);
                            spawn_local(clone!(app => async move {
                                App::read_degiro_csv(app, read_as_bytes(&blob).await.unwrap());
                            }));
                        }))
                    })
                })
            )
        })
    }

    fn render_ib_html_input(app: Arc<Self>) -> Dom {
        html!("span", {
            .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
            .child(Tooltip::render(app.ib_tooltip.clone(),
                html!("p", {
                    .text(" Para descargar el informe anual de interactive brokers: ")
                }),
                html!("ul", {
                    .children(&mut [
                        html!("li", {
                            .text("Entre en la página de interactive brokers con su usuario.")
                        }),
                        html!("li", {
                            .text("En el menú superior seleccione Rendimientos e informes y seguidamente extractos.")
                        }),
                        html!("li", {
                            .text("En extractos predeterminados pulse en actividad, seleccione el período anual, el formato HTML/descargar y en opciones Inglés.")
                        }),
                        html!("li", {
                            .text("Pulse ejecutar.")
                        }),
                    ])
                }))
            )
            .child(
                html!("input" => HtmlInputElement, {
                    .attr("id", "ib_html_report")
                    .attr("alt", "Fichero HTML comprimido informe Interactive Brokers")
                    .attr("accept", "text/html")
                    .attr("type", "file")
                    .with_node!(element => {
                        .event(clone!(app => move |_: events::Change| {
                            let file_list = match element.files() {
                                Some(file_list) => file_list,
                                None => {
                                    *app.current_error.lock_mut() = Some(
                                    "Error subiendo fichero HTML de interactive brokers".to_string());
                                    return;
                                }
                            };
                            let ib_html_data = match file_list.get(0) {
                                Some(data) => data,
                                None => {
                                    *app.current_error.lock_mut() = Some(
                                    "Error obteniendo HTML de interactive brokers".to_string());
                                    return;
                                }
                            };
                            let blob = Blob::from(ib_html_data);
                            spawn_local(clone!(app => async move {
                                App::read_ib_html(app, read_as_bytes(&blob).await.unwrap());
                            }));
                        }))
                    })
                })
            )
        })
    }

    fn render_ib_csv_input(app: Arc<Self>) -> Dom {
        html!("span", {
            .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
            .child(Tooltip::render(app.ib_csv_tooltip.clone(),
                html!("p", {
                    .text(" Para descargar el informe anual en formato CSV de interactive brokers: ")
                }),
                html!("ul", {
                    .children(&mut [
                        html!("li", {
                            .text("Entre en la página de interactive brokers con su usuario.")
                        }),
                        html!("li", {
                            .text("En el menú superior seleccione Rendimientos e informes y seguidamente extractos.")
                        }),
                        html!("li", {
                            .text("En extractos predeterminados pulse en actividad, seleccione el período anual, el formato CSV y en opciones Inglés.")
                        }),
                        html!("li", {
                            .text("Pulse ejecutar.")
                        }),
                    ])
                }))
            )
            .child(
                html!("input" => HtmlInputElement, {
                    .attr("id", "ib_csv_report")
                    .attr("alt", "Fichero CSV informe Interactive Brokers")
                    .attr("accept", "text/csv")
                    .attr("type", "file")
                    .with_node!(element => {
                        .event(clone!(app => move |_: events::Change| {
                            let file_list = match element.files() {
                                Some(file_list) => file_list,
                                None => {
                                    *app.current_error.lock_mut() = Some(
                                    "Error subiendo fichero CSV de interactive brokers".to_string());
                                    return;
                                }
                            };
                            let ib_csv_data = match file_list.get(0) {
                                Some(data) => data,
                                None => {
                                    *app.current_error.lock_mut() = Some(
                                    "Error obteniendo CSV de interactive brokers".to_string());
                                    return;
                                }
                            };
                            let blob = Blob::from(ib_csv_data);
                            spawn_local(clone!(app => async move {
                                App::read_ib_csv(app, read_as_bytes(&blob).await.unwrap());
                            }));
                        }))
                    })
                })
            )
        })
    }

    fn render_brokers_form(app: Arc<Self>) -> Dom {
        html!("section", {
            .class(&*FLEX_CONTAINER_CLASS)
            .children(&mut [
                html!("img", {
                    .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
                    .attr("src", "img/degiro.svg")
                    .attr("alt", "logo broker Degiro")
                    .attr("width", "70")
                    .attr("height", "70")
                }),
                html!("label", {
                    .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
                    .attr("for", "degiro_pdf_report")
                    .text("Informe anual broker Degiro (PDF):")
                }),
                App::render_degiro_pdf_input(app.clone()),
                html!("label", {
                    .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
                    .attr("for", "degiro_csv_report")
                    .text("Informe anual broker Degiro (CSV):")
                }),
                App::render_degiro_csv_input(app.clone()),
                html!("img", {
                    .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
                    .attr("src", "img/interactive_brokers.svg")
                    .attr("alt", "logo interactive brokers")
                    .attr("width", "70")
                    .attr("height", "70")
                }),
                html!("label", {
                    .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
                    .attr("for", "ib_html_report")
                    .text("Informe anual Interactive Brokers (HTML):")
                }),
                App::render_ib_html_input(app.clone()),
                html!("label", {
                    .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
                    .attr("for", "ib_csv_report")
                    .text("Informe anual Interactive Brokers (CSV):")
                }),
                App::render_ib_csv_input(app),
            ])
        })
    }

    fn render_account_note(note: &AccountNote) -> Dom {
        html!("tr", {
            .children(& mut[
                html!("td", {
                    .text(&note.broker.name)
                }),
                html!("td", {
                    .text(&note.company.name)
                }),
                html!("td", {
                    .text(&note.company.isin)
                }),
                html!("td", {
                    .text(&note.value.to_string())
                }),
            ])
        })
    }

    fn render_account_notes(app: Arc<Self>) -> Dom {
        html!("table", {
            .class(&*FLEX_CONTAINER_ITEM_40_CLASS)
            .children(&mut [
                html!("caption", {
                    .text("Movimientos brokers")
                }),
                html!("thead", {
                    .child(
                        html!("tr", {
                            .children(&mut [
                                html!("th", {
                                    .text("Broker")
                                }),
                                html!("th", {
                                    .text("Acción")
                                }),
                                html!("th", {
                                    .text("ISIN")
                                }),
                                html!("th", {
                                    .text("Valor (€)")
                                }),
                            ])
                    }))
                }),
            ])
            .child(html!("tbody", {
                .children_signal_vec(app.account_notes.signal_vec_cloned()
                    .map(|note| {
                        App::render_account_note(&note)
                    })
                )
            }))
        })
    }

    fn render_balance_note(note: &BalanceNote) -> Dom {
        html!("tr", {
            .children(& mut[
                html!("td", {
                    .text(&note.broker.name)
                }),
                html!("td", {
                    .text(&note.company.name)
                }),
                html!("td", {
                    .text(&note.company.isin)
                }),
                html!("td", {
                    .text(&note.value_in_euro.to_string())
                }),
            ])
        })
    }

    fn render_balance_notes(app: Arc<Self>) -> Dom {
        html!("table", {
            .class(&*FLEX_CONTAINER_ITEM_40_CLASS)
            .children(&mut [
                html!("caption", {
                    .text("Balance brokers")
                }),
                html!("thead", {
                    .child(
                        html!("tr", {
                            .children(&mut [
                                html!("th", {
                                    .text("Broker")
                                }),
                                html!("th", {
                                    .text("Acción")
                                }),
                                html!("th", {
                                    .text("ISIN")
                                }),
                                html!("th", {
                                    .text("Valor (€)")
                                }),
                            ])
                    }))
                }),
            ])
            .child(html!("tbody", {
                .children_signal_vec(app.balance_notes.signal_vec_cloned()
                    .map(|note| {
                        App::render_balance_note(&note)
                    })
                )
            }))
        })
    }

    fn render_personal_info(app: Arc<Self>) -> Dom {
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
                            .event(clone!(app => move |_: events::Input| {
                                *app.name.lock_mut() = element.value().to_uppercase();
                                App::generate_720_file(app.clone());
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
                            .event(clone!(app => move |_: events::Input| {
                                *app.surname.lock_mut() = element.value().to_uppercase();
                                App::generate_720_file(app.clone());
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
                            .event(clone!(app => move |_: events::Input| {
                                *app.nif.lock_mut() = element.value().to_uppercase();
                                App::generate_720_file(app.clone());
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
                            .event(clone!(app => move |_: events::Input| {
                                *app.year.lock_mut() = element.value().parse::<usize>().unwrap_or(DEFAULT_YEAR);
                                App::generate_720_file(app.clone());
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
                            .event(clone!(app => move |_: events::Input| {
                                *app.phone.lock_mut() = element.value().to_uppercase();
                                App::generate_720_file(app.clone());
                            }))
                        })
                    }),
                ])
            }))
        })
    }

    fn render_financial_information(app: Arc<Self>) -> Dom {
        html!("section", {
            .class(&*FLEX_CONTAINER_CLASS)
            .child(
                App::render_balance_notes(app.clone())
            )
            .child(html!("p", {
                .class(&*FLEX_CONTAINER_ITEM_20_CLASS)
            }))
            .child(
                App::render_account_notes(app)
            )
        })
    }

    fn render_download_button(app: Arc<Self>) -> Dom {
        html!("section", {
            .style("text-align", "center")
            .child_signal(app.aeat720_form_path.signal_ref(|x| x.clone()).map(move |url| {
                match url {
                    Some(path) => Some(html!("a",{
                        .attr("id", "aeat_720_form")
                        .attr("href", &path)
                        .attr("alt", "Informe 720 generado")
                        .attr("download", "fichero-720.txt")
                        .child(html!("button", {
                            .attr("type", "button")
                            .text("Descargar informe AEAT 720")
                        }))
                    })),
                    None => Some(html!("button", {
                        .attr("type", "button")
                        .attr("disabled", "true")
                        .text("Descargar informe AEAT 720")
                    })),
                }
            }))
        })
    }

    pub fn render(app: Arc<Self>) -> Dom {
        stylesheet!("html", {
            .style("font-family", "arial")
        });

        html!("div", {
            .class(&*ROOT_CLASS)
            .child_signal(app.current_error.signal_ref(|x| x.clone()).map(|text| {
                text.map(|txt|
                    html!("p", {
                        .class(&*ERROR_PARAGRAPH_CLASS)
                        .text(&txt)
                    })
                )
            }))
            .children(&mut [
                html!("h3", {
                    .class(&*SECTION_HEADER)
                    .text(" Información brokers ")
                }),
                App::render_brokers_form(app.clone()),
                html!("h3", {
                    .class(&*SECTION_HEADER)
                    .text(" Información personal ")
                }),
                App::render_personal_info(app.clone()),
                html!("h3", {
                    .class(&*SECTION_HEADER)
                    .text(" Movimientos importados ")
                }),
                App::render_financial_information(app.clone()),
                html!("h3", {
                    .class(&*SECTION_HEADER)
                    .text(" Descarga de formulario 720 ")
                }),
                App::render_download_button(app),
                html!("hr", {}),
                crate::footer::render_footer(),
            ])

        })
    }
}
