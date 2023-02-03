#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#[allow(unused_imports)]
use anyhow::Result;
use rvcd::Rvcd;
use tracing::info;
use rvcd::app::RvcdApp;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> Result<()> {
    use rvcd::server::server::rvcd_rpc_server::RvcdRpcServer;
    use rvcd::server::RvcdRemote;
    use tonic::transport::Server;

    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    // let native_options = eframe::NativeOptions::default();
    let native_options = eframe::NativeOptions {
        drag_and_drop_support: true,
        initial_window_size: Some([1280.0, 1024.0].into()),
        // #[cfg(feature = "wgpu")]
        // renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    let gui = async move {
        eframe::run_native(
            "Rvcd",
            native_options,
            Box::new(|cc| Box::new(RvcdApp::new(cc))),
        );
    };
    let rpc = async move {
        let addr = "[::1]:50051".parse().unwrap();
        info!("starting rpc server at {}", addr);
        Server::builder()
            .add_service(RvcdRpcServer::new(RvcdRemote::default()))
            .serve(addr)
            .await
            .unwrap();
    };
    // pin_mut!(gui, rpc);
    // let _ = select(gui, rpc).await;
    tokio::spawn(rpc);
    gui.await;
    Ok(())
}

// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    info!("starting rvcd");

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "the_canvas_id", // hardcode it
            web_options,
            Box::new(|cc| Box::new(Rvcd::new(cc))),
        )
        .await
        .expect("failed to start eframe");
    });
}
