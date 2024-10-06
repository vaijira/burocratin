use std::{sync::Arc, usize};

use dominator::{clone, events, html, with_node, Dom};
use futures_signals::{
    signal::{Mutable, SignalExt},
    signal_vec::{MutableVec, SignalVecExt},
};
use web_sys::HtmlElement;

use crate::{data::Aeat720Record, utils::usize_to_date};

pub struct Table {
    headers: Vec<&'static str>,
    data: MutableVec<Aeat720Record>,
    selected_row: Mutable<Option<usize>>,
}

impl Table {
    pub fn new(aeat720_records: MutableVec<Aeat720Record>) -> Arc<Self> {
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
            data: aeat720_records,
            selected_row: Mutable::new(None),
        })
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
                  .text(" ")
                })
              )
              .children(Self::render_header_cells(this))
            })
          )
        })
    }

    fn render_row(this: &Arc<Self>, index: usize, data: &Aeat720Record) -> Dom {
        let date = usize_to_date(data.first_tx_date)
            .map_or("".to_string(), |d| d.format("%d/%m/%Y").to_string());

        html!("tr", {
          .style_signal("background-color", this.selected_row.signal().map(
            move |row| if row == Some(index) {
              "#ddd"
            } else {
              "#fff"
            }
          ))
          .children(&mut [
            html!("td" => HtmlElement, {
              .style("font-weight", "bold")
              .style("background-color", "#ddd")
              .text(" ")
              .with_node!(_element => {
                .event(clone!(this => move |_: events::Click| {
                  if this.selected_row.get() == Some(index) {
                    this.selected_row.set(None)
                  } else {
                    this.selected_row.set(Some(index))
                  }
                }))
              })
            }),
            html!("td", {
              .text(&data.company.name)
            }),
            html!("td", {
              .text(&data.company.isin)
            }),
            html!("td", {
              .text(&data.broker.country_code)
            }),
            html!("td", {
              .text(&date)
            }),
            html!("td", {
              .text(&data.value_in_euro.to_string())
            }),
            html!("td", {
              .text(&data.quantity.to_string())
            }),
            html!("td", {
              .text("100%")
            }),

          ])
        })
    }

    fn render_body(this: &Arc<Self>) -> Dom {
        html!("tbody", {
          .children_signal_vec(this.data.signal_vec_cloned()
            .enumerate().map(clone!(this => move |(index, record)| {
              Table::render_row(&this, index.get().unwrap_or(usize::MAX), &record)
            }))
          )
        })
    }

    pub fn render(this: &Arc<Self>) -> Dom {
        html!("table", {
          .style("overflow", "auto")
          .style("width", "100%")
          .style("height", "400px")
          .style("border-collapse", "collapse")
          .style("border", "1px solid #8c8c8c")
          .style("margin-bottom" ,"1em")
          .child(
            html!("caption", {
              .text("Movimientos importados.")
            })

          )
          .child(Self::render_header(this))
          .child(Self::render_body(this))
        })
    }
}
