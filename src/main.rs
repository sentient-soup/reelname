mod app;
mod core;
mod db;
mod theme;
mod tray;
mod ui;

fn main() -> iced::Result {
    // Initialize logging (simple, no env-filter feature needed)
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("ReelName starting...");

    // Run the Iced application
    // boot is the first arg (returns (State, Task)), then update, then view
    iced::application(app::App::new, app::App::update, app::App::view)
        .subscription(app::App::subscription)
        .theme(app::App::theme)
        .window_size((1280.0, 800.0))
        .antialiasing(true)
        .run()
}
