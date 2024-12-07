use std::sync::Arc;

use chrono::NaiveDate;
use dominator::{clone, events, html, with_node, Dom};
use futures_signals::{
    map_ref,
    signal::{Mutable, Signal, SignalExt},
    signal_vec::{MutableVec, SignalVecExt},
};
use web_sys::{HtmlElement, HtmlInputElement};

use crate::{
    css::{TABLE_CAPTION, TABLE_ROW, TABLE_STYLE},
    data::{Aeat720Record, BrokerInformation},
    utils::{icons::render_svg_trash_icon, usize_to_date},
};

const ISIN_NOT_VALID_ERR_MSG: &str = "ISIN no válido";

#[derive(Debug, Clone)]
struct Aeat720RecordInfo {
    record: Aeat720Record,
    isin_err_msg: Mutable<Option<&'static str>>,
}
pub struct Table {
    headers: Vec<&'static str>,
    data: MutableVec<Mutable<Aeat720RecordInfo>>,
}

impl Table {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            headers: vec![
                "Nombre compañía",
                "ISIN",
                "Código país",
                "Fecha 1ª adquisición",
                "Valor Euros",
                "Nº acciones",
                "Porcentaje",
            ],
            data: MutableVec::new(),
        })
    }

    pub fn table_rows_not_empty(&self) -> impl Signal<Item = bool> {
        self.data
            .signal_vec_cloned()
            .to_signal_map(|x| !x.is_empty())
    }

    pub fn extend_rows(&self, records: Vec<Aeat720Record>) {
        for record in records.into_iter() {
            self.data
                .lock_mut()
                .push_cloned(Mutable::new(Aeat720RecordInfo {
                    record,
                    isin_err_msg: Mutable::new(None),
                }));
        }
    }

    pub fn get_records(&self) -> Vec<Aeat720Record> {
        let mut result = vec![];
        for record in self.data.lock_ref().iter() {
            result.push(record.lock_ref().record.clone());
        }
        result
    }

    pub fn clear(&self) {
        self.data.lock_mut().clear();
    }

    fn render_header_cells(this: &Arc<Self>) -> Vec<Dom> {
        this.headers
            .iter()
            .map(|header_cell| {
                html!("th", {
                  .attr("scope", "col")
                  .style("vertical-align", "bottom")
                  .style("font-weight", "bold")
                  .style("background-color", "#ddd")
                  .text(header_cell)
                })
            })
            .collect()
    }

    fn render_header(this: &Arc<Self>) -> Dom {
        html!("thead", {
          .child(
            html!("tr", {
              .child(
                html!("th", {
                  .attr("scope", "col")
                  .style("vertical-align", "bottom")
                  .style("font-weight", "bold")
                  .style("background-color", "#ddd")
                  .text("#")
                })
              )
              .children(Self::render_header_cells(this))
              .child(
                html!("th", {
                  .attr("scope", "col")
                  .style("vertical-align", "bottom")
                  .style("font-weight", "bold")
                  .style("background-color", "#ddd")
                  .text("")
                })
              )
            })
          )
        })
    }

    fn company_name_cell(record: &Mutable<Aeat720RecordInfo>) -> impl Signal<Item = Option<Dom>> {
        record.signal_ref(clone!(record => move |r| {
          let err_msg: Mutable<Option<&str>> = Mutable::new(None);
          let name_error_msg = "Nombre no válido";
            Some(
              html!("td", {
                .child(
                  html!("input" => HtmlInputElement, {
                    .style("display", "block")
                    .attr("type", "text")
                    .attr("size", "40")
                    .attr("maxlength", "40")
                    .attr("value", &r.record.company.name)
                    .with_node!(element => {
                      .event(clone!(record, err_msg => move |_: events::Change| {
                        let name = element.value();
                        if !name.is_empty() {
                          *err_msg.lock_mut() = None;
                        } else {
                          *err_msg.lock_mut() = Some(name_error_msg);
                          let _ = element.focus();
                        }
                        record.lock_mut().record.company.name = name;
                      }))
                    })
                  })
                )
                .child(
                  html!("span", {
                    .style("color", "red")
                    .style("font-size", "small")
                    .text_signal(err_msg.signal_ref(|t| t.unwrap_or("")))
                  })
                )
              })
            )
        }))
    }

    fn company_isin_cell(record: &Mutable<Aeat720RecordInfo>) -> impl Signal<Item = Option<Dom>> {
        record.signal_ref(clone!(record => move |r| {
            Some(
              html!("td", {
                .child(
                  html!("input" => HtmlInputElement, {
                    .style("display", "block")
                    .attr("type", "text")
                    .attr("size", "12")
                    .attr("maxlength", "12")
                    .attr("value", &r.record.company.isin)
                    .with_node!(element => {
                      .event(clone!(record => move |_: events::Change| {
                        let isin = element.value();
                        if isin::parse(&isin).is_ok() {
                          *record.lock_mut().isin_err_msg.lock_mut() = None;
                        } else {
                          *record.lock_mut().isin_err_msg.lock_mut() = Some(ISIN_NOT_VALID_ERR_MSG);
                          let _ = element.focus();
                        }
                        record.lock_mut().record.company.isin = isin;
                      }))
                    })
                  })
                )
                .child(
                  html!("span", {
                    .style("color", "red")
                    .style("font-size", "small")
                    .text_signal(record.lock_ref().isin_err_msg.signal_ref(|t| t.unwrap_or("")))
                  })
                )
              })
            )
        }))
    }

    fn broker_country_code_cell(
        record: &Mutable<Aeat720RecordInfo>,
    ) -> impl Signal<Item = Option<Dom>> {
        record.signal_ref(clone!(record => move |r| {
            Some(
              html!("td", {
                .child(
                  html!("input" => HtmlInputElement, {
                    .attr("type", "text")
                    .attr("size", "2")
                    .attr("maxlength", "2")
                    .attr("value", &r.record.broker.country_code)
                    .with_node!(element => {
                      .event(clone!(record => move |_: events::Change| {
                        let broker = Arc::new(BrokerInformation{
                          name: "new unknown".to_string(),
                          country_code: element.value(),
                        });
                        record.lock_mut().record.broker = broker;
                      }))
                    })
                  })
                )
              })
            )
        }))
    }

    fn date_cell(record: &Mutable<Aeat720RecordInfo>) -> impl Signal<Item = Option<Dom>> {
        record.signal_ref(clone!(record => move |r| {
          let first_tx_date = r.record.first_tx_date;
          let date = usize_to_date(first_tx_date)
              .map_or("".to_string(), |d| d.format("%Y-%m-%d").to_string());
            Some(
              html!("td", {
                .child(html!("input" => HtmlInputElement, {
                  .attr("type", "date")
                  .attr("value", &date)
                  .with_node!(element => {
                      .event(clone!(record => move |_: events::Change| {
                        let parsed_date = NaiveDate::parse_from_str(&element.value(), "%Y-%m-%d").unwrap();
                        record.lock_mut().record.first_tx_date =
                          parsed_date.format("%Y%m%d").to_string().parse::<usize>().unwrap_or(first_tx_date);
                      }))
                    })
                }))
              })
            )
        }))
    }

    fn value_cell(record: &Mutable<Aeat720RecordInfo>) -> impl Signal<Item = Option<Dom>> {
        record.signal_ref(move |r| {
            Some(html!("td", {
              .child(html!("input", {
                .style("text-align", "right")
                .attr("type", "text")
                .attr("size", "15")
                .attr("maxlength", "15")
                .attr("value", &r.record.value_in_euro.to_string())
              }))
            }))
        })
    }

    fn quantity_cell(record: &Mutable<Aeat720RecordInfo>) -> impl Signal<Item = Option<Dom>> {
        record.signal_ref(move |r| {
            Some(html!("td", {
              .child(html!("input", {
                .style("text-align", "right")
                .attr("type", "text")
                .attr("size", "15")
                .attr("maxlength", "15")
                .attr("value", &r.record.quantity.to_string())
              }))
            }))
        })
    }

    fn percentage_cell(record: &Mutable<Aeat720RecordInfo>) -> impl Signal<Item = Option<Dom>> {
        record.signal_ref(move |r| {
            Some(html!("td", {
              .child(html!("input", {
                .style("text-align", "right")
                .attr("type", "text")
                .attr("size", "6")
                .attr("maxlength", "6")
                .attr("value", &r.record.percentage.to_string())
              }))
              .text("%")
            }))
        })
    }

    fn actions_cell(
        this: &Arc<Self>,
        index: usize,
        record: &Mutable<Aeat720RecordInfo>,
    ) -> impl Signal<Item = Option<Dom>> {
        record.signal_ref(clone!(this => move |_r| {
         let delete_span = html!("span" => HtmlElement, {
           .child(render_svg_trash_icon("red", "24"))
          .with_node!(_element => {
            .event(clone!(this => move |_: events::Click| {
              this.data.lock_mut().remove(index);
            }))
          })
         });

            Some(
              html!("td", {
                .child(delete_span)
              })
            )
        }))
    }

    fn render_row(this: &Arc<Self>, index: usize, record: &Mutable<Aeat720RecordInfo>) -> Dom {
        html!("tr", {
          .class(&*TABLE_ROW)
          .child(
            html!("td", {
              .text(&format!("{}", index + 1))
            })
          )
          .child_signal(Self::company_name_cell(record))
          .child_signal(Self::company_isin_cell(record))
          .child_signal(Self::broker_country_code_cell(record))
          .child_signal(Self::date_cell(record))
          .child_signal(Self::value_cell(record))
          .child_signal(Self::quantity_cell(record))
          .child_signal(Self::percentage_cell(record))
          .child_signal(Self::actions_cell(this, index, record))
        })
    }

    fn render_body(this: &Arc<Self>) -> Dom {
        html!("tbody", {
          .children_signal_vec(this.data.signal_vec_cloned()
            .enumerate().map(clone!(this => move |(index, record)| {
              let i = index.get().unwrap_or(usize::MAX);
              Table::render_row(&this, i, &record)
           }))
          )
        })
    }

    fn is_needed_to_rerender_rows(this: &Arc<Self>) -> impl Signal<Item = bool> {
        map_ref! {
            // let _editable_changed = this.editable.signal(),
            let records_len = this.data.signal_vec_cloned().to_signal_map(|x| x.len()) => {
              log::debug!("Rerendering rows, new rows: {}", records_len);
              true
            }
        }
    }

    pub fn render(this: &Arc<Self>) -> Dom {
        html!("table", {
         .class(&*TABLE_STYLE)
         .child(
            html!("caption", {
              .class(&*TABLE_CAPTION)
              .text("Movimientos importados/creados.")
            })

          )
          .child(Self::render_header(this))
          .child_signal(Self::is_needed_to_rerender_rows(this).map(
            clone!(this => move |_x| {
              Some(Self::render_body(&this))
            }))
          )
        })
    }
}
