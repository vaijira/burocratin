use std::sync::Arc;

use dominator::{clone, events, html, Dom};
use futures_signals::signal::{Mutable, SignalExt};

use crate::{
    css::{DEFAULT_ICON_COLOR, DEFAULT_ICON_SIZE, TOOLTIP_CONTAINER, TOOLTIP_ITEM},
    feathers::render_svg_help_icon,
};

pub struct Tooltip {
    tooltip_active: Mutable<bool>,
}

impl Tooltip {
    pub fn new() -> Arc<Self> {
        Arc::new(Tooltip {
            tooltip_active: Mutable::new(false),
        })
    }

    pub fn render(tooltip: Arc<Self>, header: Dom, text: Dom) -> Dom {
        html!("span", {
            .class(&*TOOLTIP_CONTAINER)
            .child(html!("p", {
                .class(&*TOOLTIP_ITEM)
                .style_signal("visibility",  tooltip.tooltip_active.signal().map(|v| {
                    if v { "visible" } else { "hidden" }
                }))
                .style_signal("opacity", tooltip.tooltip_active.signal().map(|v| {
                    if v { "1" } else { "0" }
                }))
                .child(header)
                .child(text)
            }))
            .child(render_svg_help_icon(DEFAULT_ICON_COLOR, DEFAULT_ICON_SIZE))
            .event(clone!(tooltip => move |_: events::PointerEnter| {
                *tooltip.clone().tooltip_active.lock_mut() = true;
            }))
            .event(clone!(tooltip => move |_: events::PointerOver| {
                *tooltip.clone().tooltip_active.lock_mut() = true;
            }))
            .event(clone!(tooltip => move |_: events::PointerLeave| {
                *tooltip.clone().tooltip_active.lock_mut() = false;
            }))
        })
    }
}
