don't use global_event
[10:29 AM]
you need to create a special modal div which has position: fixed; left: 0px; top: 0px; width: 100%; height: 100%;

2
[10:29 AM]
and then put the click event onto that div
[10:30 AM]
(and yes, that behavior is expected, since global_event puts the listener onto the window, and so you're clicking on the window... then dragging the mouse... then releasing on the window)
[10:31 AM]
here's an example from tab organizer: https://github.com/Pauan/tab-organizer/blob/f97823184ec02f1b1b23eee269baa7d621b43139/src/lib.rs#L170-L176
[10:32 AM]
then you'd do something like this...
[10:33 AM]
html!("div", {
    .class(&*MODAL_STYLE)

    .visible_signal(modal_visible.signal())

    .event(clone!(modal_visible => move |_: events::Click| {
        modal_visible.set_neq(false);
    }))
})
[10:34 AM]
(of course you can call your state.close() inside of the event as well)