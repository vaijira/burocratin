use std::sync::Arc;

use anyhow::Result;
use dominator::{clone, events, html, with_node, Dom};
use futures_signals::{
    map_ref,
    signal::{Mutable, Signal, SignalExt},
};
use gloo_file::{futures::read_as_bytes, Blob};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Element, HtmlAnchorElement, HtmlElement, HtmlInputElement};

use crate::{
    data::{Aeat720Information, PersonalInformation},
    personal_info::PersonalInfoViewer,
    table::Table,
    utils::{file_importer, web},
};

pub struct App {
    current_error: Mutable<Option<String>>,
    personal_info: Mutable<PersonalInformation>,
    aeat720_form_path: Mutable<Option<String>>,
    personal_info_viewer: Arc<PersonalInfoViewer>,
    table: Arc<Table>,
}

impl App {
    pub fn new() -> Arc<Self> {
        let personal_info = Mutable::new(PersonalInformation::default());

        Arc::new(Self {
            current_error: Mutable::new(None),
            personal_info: personal_info.clone(),
            aeat720_form_path: Mutable::new(None),
            personal_info_viewer: PersonalInfoViewer::new(personal_info.clone()),
            table: Table::new(),
        })
    }

    fn is_needed_to_generate_report(this: &Arc<Self>) -> impl Signal<Item = bool> {
        map_ref! {
            let _personal_info_changed = this.personal_info.signal_ref(|_| true),
            let records_changed = this.table.table_rows_not_empty() =>
            *records_changed // || *personal_info_changed
        }
    }

    fn import_file(this: &Arc<Self>, content: Vec<u8>) {
        let import_data = file_importer(content);
        match import_data {
            Ok(records) => {
                this.table.extend_rows(records);
            }
            Err(error) => {
                *this.current_error.lock_mut() = Some(error.to_string());
            }
        }
    }

    fn generate_720_file(this: &Arc<Self>) -> Result<()> {
        let old_path = (*this.aeat720_form_path.lock_ref()).clone();
        let old_path = old_path.map_or("".to_owned(), |x| x);
        let path = web::generate_720(&Aeat720Information {
            records: this.table.get_records(),
            personal_info: PersonalInformation::default(),
        })?;
        if !old_path.is_empty() {
            let _ = web::delete_path(old_path);
        }

        *this.aeat720_form_path.lock_mut() = Some(path);
        Ok(())
    }

    fn render_import_button(this: &Arc<Self>) -> Dom {
        html!("span", {
          .child(
            html!("button", {
              .child(
                html!("label", {
                  .style("cursor", "pointer")
                  .attr("autofocus", "autofocus")
                  .attr("for", "import_report")
                  .text("Importar informes de brokers")
              })
            )})
          )
          .child(
            html!("input" => HtmlInputElement, {
              .attr("id", "import_report")
              .attr("alt", "Botón para importar ficheros de Interactive brokers o Degiro")
              .attr("accept", "text/html,text/csv,application/pdf,application/zip,.zip,.pdf,.csv,.html")
              .attr("type", "file")
              .style("display", "none")
              .with_node!(element => {
                .event(clone!(this => move |_: events::Change| {
                    let file_list = match element.files() {
                    Some(file_list) => file_list,
                    None => {
                      *this.current_error.lock_mut() = Some(
                        "Error subiendo fichero".to_string());
                      return;
                    }
                  };
                  let file_data = match file_list.get(0) {
                    Some(data) => data,
                    None => {
                      *this.current_error.lock_mut() = Some(
                        "Error obteniendo fichero".to_string());
                      return;
                    }
                  };
                  let blob = Blob::from(file_data);
                  spawn_local(clone!(this => async move {
                    App::import_file(&this, read_as_bytes(&blob).await.unwrap());
                  }));
                  element.set_value("");
                }))
              })
            })
          )
        })
    }

    fn render_clear_button(this: &Arc<Self>) -> Dom {
        html!("span", {
          .child(html!("input" => HtmlInputElement, {
            .attr("type", "button")
            .attr("value", "Limpiar movimientos")
            .with_node!(_element => {
              .event(clone!(this => move |_: events::Click| {
                this.table.clear();
              }))
            })
          }))
        })
    }

    fn render_download_button(this: &Arc<Self>) -> Dom {
        html!("section", {
         .child_signal(
           Self::is_needed_to_generate_report(this).map(clone!(this => move |x| {
              if x {
                  Some(
                    html!("button" => HtmlElement, {
                      .attr("type", "button")
                      .text("Descargar informe AEAT 720")
                      .with_node!(_element => {
                        .event(clone!(this => move |_: events::Click| {
                          let result = App::generate_720_file(&this);
                          if result.is_ok() {
                            let file_path = this.aeat720_form_path.lock_ref().clone().unwrap();
                            let elem: Element = gloo_utils::document().create_element("a").unwrap_throw();
                            let link: HtmlAnchorElement = elem.dyn_into().unwrap_throw();
                            link.set_href(&file_path);
                            let _ = link.set_attribute("download", "fichero-720.txt");
                            link.click();
                            /* let file_path = this.aeat720_form_path.lock_ref().clone().unwrap();
                            let _ = web_sys::window().unwrap_throw().open_with_url_and_target(&file_path, "_self"); */
                          }
                        }))
                      })
                    })
                  )
             } else {
               Some(
                html!("button", {
                  .attr("type", "button")
                  .attr("disabled", "true")
                  .text("Descargar informe AEAT 720")
                }))
             }
          })))
        })
    }

    pub fn render(this: Arc<Self>) -> Dom {
        html!("div", {
            .child(html!("h3", {
                .text("Paso 1: Rellena datos personales.")
            }))
            .child(PersonalInfoViewer::render(&this.personal_info_viewer))
            .child(html!("h3", {
                .text("Paso 2: Descarga los informes de Interactive brokers y/o Degiro e importalos.")
            }))
            .child(
               Table::render(&this.table)
            )
            .child(
                App::render_import_button(&this)
            )
            .child(
                App::render_clear_button(&this)
            )
            .child(html!("h3", {
                .text("Paso 3: Revisa las fechas de 1º adquisición y los datos importados y descarga el fichero generado.")
            }))
            .child(App::render_download_button(&this))
            .child(html!("h3", {
                .text("Paso 4: Finalmente importe el fichero descargado con el modelo 720 en la ")
                .child(html!("a", {
                  .attr("alt", "enlace presentación modelo 720 AEAT")
                  .attr("target", "_blank")
                  .attr("rel", "noopener external nofollow")
                  .attr("href", "https://sede.agenciatributaria.gob.es/Sede/procedimientoini/GI34.shtml")
                  .text("página correspondiente de la AEAT")
                }))
                .text(" y revise el código de domiciliación del país de las empresas, por defecto cogerá el del ISIN, pero esto no siempre es correcto.")
            }))
        //<p>Finalmente suba el fichero descargado con el modelo 720 a <a alt="enlace modelo 720 AEAT" target="_blank" rel="noopener external nofollow"
        // href="https://sede.agenciatributaria.gob.es/Sede/procedimientoini/GI34.shtml">página correspondiente de la AEAT</a> y comparta en redes sociales si le ha resultado de utilidad.</p>
        })
    }
}
