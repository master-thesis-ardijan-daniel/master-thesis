use gloo::console;
use gloo::events::EventListener;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use wasm_bindgen::JsCast;
use web_sys::{EventTarget, MouseEvent};

pub struct MouseMove {
    receiver: UnboundedReceiver<()>,
    listener: EventListener,
}

impl MouseMove {
    pub fn new(target: &EventTarget) -> Self {
        let (_sender, receiver) = mpsc::unbounded_channel();

        let listener = EventListener::new(target, "mousemove", |event| {
            let event = event.unchecked_ref::<MouseEvent>();

            console::log!("Mouse position: (", event.x(), ", ", event.y(), ")");
        });

        Self { receiver, listener }
    }
}
