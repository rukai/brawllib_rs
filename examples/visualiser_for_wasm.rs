#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Use the run_web_visualiser.sh script to run this example");
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
    use brawllib_rs::renderer::app::state::{AppEvent, State};
    use brawllib_rs::renderer::app::App;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::HtmlElement;

    let document = web_sys::window().unwrap().document().unwrap();
    let visualiser_span = document.get_element_by_id("fighter-render").unwrap();

    let app = App::new_insert_into_element(visualiser_span, subaction).await;
    let event_tx = app.get_event_tx();

    // TODO:
    // hmmmmmmmmmmmmmmmmmm
    // It would be ideal if we could completely control the app from here by just sending events.
    // Two way communication would add a lot of complexity.
    // The tricky part is how to handle updating the current frame.
    // Can we do that without access to the render loop?

    // possible approaches:
    // ## directly use surface?
    // * yew
    // * how do I get a surface for wgpu
    //
    // ## winit + reuse app + move rukaidata UI into brawllib via iced
    // * send state in with event
    // * AppState just stores state and everyone is responsible for sending in AppEvent to update it.
    //
    // ## winit + reuse App/AppState
    // * send state in with event
    // * AppState just stores state and everyone is responsible for sending in AppEvent to update it.
    //
    // ## winit + dont reuse App/AppState
    // * reimplement equivalent of App for maximum flexibility
    //
    // wow I really cant figure out which one I want.
    // Im thinking:
    // 1. get this running in rukaidata so I can test things properly
    // 2. go with: "winit + dont reuse app", then prototype really quickly to see if it works.
    // 3. move on with my life. I really dont think I care about having to focus the app to give keyboard inputs.

    let frame = document.get_element_by_id("frame").unwrap();
    let frame_move = frame.clone();
    frame_move.set_inner_html("Frame: 0");

    let button = document.get_element_by_id("run").unwrap();
    let button_move = button.clone();
    button_move.set_inner_html("Run");
    let do_thing = Closure::wrap(Box::new(move || {
        if button_move.inner_html() == "Stop" {
            event_tx.send(AppEvent::SetState(State::Pause)).unwrap();
            button_move.set_inner_html("Run");
        } else {
            event_tx.send(AppEvent::SetState(State::Play)).unwrap();
            button_move.set_inner_html("Stop");
        }
    }) as Box<dyn FnMut()>);
    button
        .dyn_ref::<HtmlElement>()
        .unwrap()
        .set_onclick(Some(do_thing.as_ref().unchecked_ref()));

    app.get_event_tx()
        .send(AppEvent::SetState(State::Pause))
        .unwrap();

    app.run();
}
