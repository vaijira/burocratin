use std::sync::Arc;

use dominator::{clone, events, html, stylesheet, with_node, Dom};
use futures_signals::{
    signal::{Mutable, SignalExt},
    signal_vec::MutableVec,
};
use gloo_file::{futures::read_as_bytes, Blob};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

use crate::{
    css::{ROOT_CLASS, SECTION_HEADER},
    data::{Aeat720Information, Aeat720Record, PersonalInformation},
    personal_info::PersonalInfoViewer,
    table::Table,
    utils::{file_importer, web},
};

pub struct App {
    current_error: Mutable<Option<String>>,
    aeat720_records: MutableVec<Aeat720Record>,
    personal_info: Mutable<PersonalInformation>,
    aeat720_form_path: Mutable<Option<String>>,
    personal_info_viewer: Arc<PersonalInfoViewer>,
    table: Arc<Table>,
}

impl App {
    pub fn new() -> Arc<Self> {
        let personal_info = Mutable::new(PersonalInformation::default());
        let aeat720_records = MutableVec::new();

        Arc::new(Self {
            current_error: Mutable::new(None),
            aeat720_records: MutableVec::new(), // aeat720_records.clone(),
            personal_info: personal_info.clone(),
            aeat720_form_path: Mutable::new(None),
            personal_info_viewer: PersonalInfoViewer::new(personal_info.clone()),
            table: Table::new(aeat720_records.clone()),
        })
    }

    fn import_file(this: &Arc<Self>, content: Vec<u8>) {
        let import_data = file_importer(content);
        match import_data {
            Ok(records) => {
                this.aeat720_records.lock_mut().extend(records);
            }
            Err(error) => {
                *this.current_error.lock_mut() = Some(error.to_string());
            }
        }
    }

    fn generate_720_file(this: &Arc<Self>) {
        let old_path = (*this.aeat720_form_path.lock_ref()).clone();
        let old_path = old_path.map_or("".to_owned(), |x| x);
        if let Ok(path) = web::generate_720(
            &Aeat720Information {
                records: this.aeat720_records.lock_ref().to_vec(),
                personal_info: PersonalInformation::default(),
            },
            &old_path,
        ) {
            *this.aeat720_form_path.lock_mut() = Some(path);
        }
    }

    fn render_import_button(this: &Arc<Self>) -> Dom {
        html!("span", {
          .style("text-align", "center")
          .child(
            html!("label", {
              .attr("for", "import_report")
              .text("Importar informes de Interactive brookers o Degiro: ")
            })
          )
          .child(
            html!("input" => HtmlInputElement, {
              .attr("id", "import_report")
              .attr("alt", "Botón para importar ficheros de Interactive brokers o Degiro")
              .attr("accept", "text/html,text/csv,application/pdf,application/zip,.zip,.pdf,.csv,.html")
              .attr("type", "file")
              .with_node!(element => {
                .event(clone!(this => move |_: events::Change| {
                    let file_list = match element.files() {
                    Some(file_list) => file_list,
                    None => {
                      *this.current_error.lock_mut() = Some(
                        "Error subiendo fichero CSV de interactive brokers".to_string());
                      return;
                    }
                  };
                  let ib_csv_data = match file_list.get(0) {
                    Some(data) => data,
                    None => {
                      *this.current_error.lock_mut() = Some(
                        "Error obteniendo CSV de interactive brokers".to_string());
                      return;
                    }
                  };
                  let blob = Blob::from(ib_csv_data);
                  spawn_local(clone!(this => async move {
                    App::import_file(&this, read_as_bytes(&blob).await.unwrap());
                    App::generate_720_file(&this);
                  }));
                }))
              })
            })
          )
        })
    }

    fn render_clear_button(this: &Arc<Self>) -> Dom {
        html!("span", {
          .style("text-align", "center")
          .child(html!("input" => HtmlInputElement, {
            .attr("type", "button")
            .attr("value", "Limpiar movimientos")
            .with_node!(_element => {
              .event(clone!(this => move |_: events::Click| {
                this.aeat720_records.lock_mut().clear();
                App::generate_720_file(&this);
              }))
            })
          }))
        })
    }

    fn render_download_button(this: &Arc<Self>) -> Dom {
        html!("section", {
            .style("text-align", "center")
            .child_signal(this.aeat720_form_path.signal_ref(|x| x.clone()).map(move |url| {
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

    pub fn render(this: Arc<Self>) -> Dom {
        stylesheet!("html", {
          .style("font-family", "arial")
        });

        html!("div", {
            .class(&*ROOT_CLASS)
            .child(
                html!("h3", {
                  .class(&*SECTION_HEADER)
                  .text(" Información brokers ")
                })
            )
            .child(
               Table::render(&this.table)
            )
            .child(
                App::render_clear_button(&this)
            )
            .child(
                App::render_import_button(&this)
            )
            .child(
                html!("h3", {
                  .class(&*SECTION_HEADER)
                  .text(" Información personal ")
                })
            )
            .child(PersonalInfoViewer::render(&this.personal_info_viewer))
            .child(
                html!("h3", {
                  .class(&*SECTION_HEADER)
                  .text(" Descarga de formulario 720 ")
                })
            )
            .child(App::render_download_button(&this))
            .child(
                html!("hr", {})
            )
            .child(
                crate::footer::render_footer()
            )

        })
    }
}
