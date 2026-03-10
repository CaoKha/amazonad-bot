mod api;
mod app;
pub mod chart;
mod components;
mod models;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
