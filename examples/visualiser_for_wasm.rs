#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Use the run_web_visualiser.sh script to run this example");
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use brawllib_rs::renderer;

    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init().expect("could not initialize logger");

    let fighter_bytes = include_bytes!("subaction_data.bin");
    let subaction = bincode::deserialize(fighter_bytes).unwrap();
    wasm_bindgen_futures::spawn_local(renderer::render_window_wasm(subaction));
}
