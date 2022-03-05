#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Run via `cargo run-wasm --example visualiser_for_wasm`");
}

#[cfg(target_arch = "wasm32")]
fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Warn).expect("could not initialize logger");

    let fighter_bytes = include_bytes!("subaction_data.bin");
    let subaction = bincode::deserialize(fighter_bytes).unwrap();
    wasm_bindgen_futures::spawn_local(render_window_wasm(subaction));
}

#[cfg(target_arch = "wasm32")]
pub async fn render_window_wasm(subaction: brawllib_rs::high_level_fighter::HighLevelSubaction) {
    use brawllib_rs::renderer::app::state::{AppEventIncoming, State};
    use brawllib_rs::renderer::app::App;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::HtmlElement;

    let document = web_sys::window().unwrap().document().unwrap();

    let body = document.body().unwrap();
    let parent_div = document.create_element("div").unwrap();
    parent_div
        .dyn_ref::<HtmlElement>()
        .unwrap()
        .style()
        .set_css_text("margin: auto; width: 50%; aspect-ratio: 4 / 3;");
    body.append_child(&parent_div).unwrap();

    let app = App::new_insert_into_element(parent_div, subaction).await;
    let event_tx = app.get_event_tx();

    let frame = document.create_element("p").unwrap();
    frame.set_inner_html("Frame: 0");
    body.append_child(&frame).unwrap();

    let button = document.create_element("button").unwrap();
    body.append_child(&button).unwrap();
    let button_move = button.clone();
    button_move.set_inner_html("Run");
    let do_thing = Closure::wrap(Box::new(move || {
        if button_move.inner_html() == "Stop" {
            event_tx
                .send(AppEventIncoming::SetState(State::Pause))
                .unwrap();
            button_move.set_inner_html("Run");
        } else {
            event_tx
                .send(AppEventIncoming::SetState(State::Play))
                .unwrap();
            button_move.set_inner_html("Stop");
        }
    }) as Box<dyn FnMut()>);
    button
        .dyn_ref::<HtmlElement>()
        .unwrap()
        .set_onclick(Some(do_thing.as_ref().unchecked_ref()));

    app.get_event_tx()
        .send(AppEventIncoming::SetState(State::Pause))
        .unwrap();

    app.run();
}
