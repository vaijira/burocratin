use std::sync::Arc;

use dominator::{html, Dom};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};

use crate::data::Aeat720Record;

pub struct Table {
    headers: Vec<&'static str>,
    data: MutableVec<Aeat720Record>,
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
        })
    }

    fn render_header_cells(this: &Arc<Self>) -> Vec<Dom> {
        this.headers
            .iter()
            .map(|header_cell| {
                html!("th", {
                  .attr("scope", "col")
                  .attr("vertical-align", "bottom")
                  .text(header_cell)
                })
            })
            .collect()
    }

    fn render_header(this: &Arc<Self>) -> Dom {
        html!("thead", {
          .child(
            html!("tr", {
              .children(Self::render_header_cells(this))
            })
          )
        })
    }

    fn render_row(data: &Aeat720Record) -> Dom {
        html!("tr", {
          .children(&mut [
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
              .text("Pending")
            }),
            html!("td", {
              .text(&data.value_in_euro.to_string())
            }),
            html!("td", {
              .text(&data.quantity.to_string())
            }),
            html!("td", {
              .text("100")
            }),

          ])
        })
    }

    fn render_body(this: &Arc<Self>) -> Dom {
        html!("tbody", {
          .children_signal_vec(this.data.signal_vec_cloned()
            .map(|record| {
              Table::render_row(&record)
            })
          )
        })
    }

    pub fn render(this: &Arc<Self>) -> Dom {
        html!("table", {
          .attr("overflow", "auto")
          .attr("width", "100%")
          .attr("max-width", "400px")
          .attr("height", "300px")
          .attr("display", "block")
          .attr("margin", "0 auto")
          .attr("border-spacing", "0")
          .attr("border-collapse", "collapse")
          .attr("border", "1px solid #8c8c8c")
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
