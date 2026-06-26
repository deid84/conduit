use conduit_headless::{routes, state};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Bind on a random loopback port synchronously — this way we know
            // the port before constructing the WebView initialization script.
            let std_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
            std_listener.set_nonblocking(true)?;
            let port = std_listener.local_addr()?.port();

            tracing::info!("conduit API listening on 127.0.0.1:{port}");

            // Hand the listener to the Tauri/tokio async runtime.
            tauri::async_runtime::spawn(async move {
                let listener = tokio::net::TcpListener::from_std(std_listener)
                    .expect("failed to convert TcpListener");
                let st = state::AppState::new();
                // In Tauri mode the WebView serves the frontend directly;
                // pass an empty path so ServeDir is never actually hit.
                let router = routes::router(st, std::path::PathBuf::new());
                axum::serve(listener, router)
                    .await
                    .expect("Axum server error");
            });

            // Create the main window with the API base URL injected before any
            // page scripts run, so window.__CONDUIT_API__ is set at SPA init time.
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::default(),
            )
            .initialization_script(&format!(
                "window.__CONDUIT_API__ = 'http://127.0.0.1:{port}';"
            ))
            .title("Conduit")
            .inner_size(1200.0, 800.0)
            .resizable(true)
            .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run conduit");
}
