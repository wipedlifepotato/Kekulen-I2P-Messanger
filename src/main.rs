use clap::Parser;
use std::sync::Arc;

mod api;
mod config;
mod kekulenprot;
mod sam;
mod tui;

use crate::config::config::Args;
use crate::kekulenprot::Protocol;

use crate::config::config::AppConfig;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let app_dir = AppConfig::get_app_dir();

    let full_file_name = format!("{}.dat", args.dat_file);

    let mut dat_path = std::path::PathBuf::from(app_dir);
    dat_path.push(&full_file_name);

    let dat_path_str = dat_path.to_string_lossy().to_string();
    dbg!(&dat_path_str);
    let protocol = if dat_path.exists() {
        println!("Loading: {}", dat_path_str);
        Arc::new(Protocol::load_profile(
            &args.dat_file.as_str(),
            &args.password,
        ))
    } else {
        println!("Creating: {}", dat_path_str);
        Arc::new(Protocol::create_profile(
            &args.dat_file.as_str(),
            &args.password,
            7,
        ))
    };

    protocol.connect_thread();
    protocol.accept_thread();

    if args.run_tui {
        tui::run(protocol);
    } else {
        let server = Arc::new(api::ApiServer::new(Arc::clone(&protocol)));
        server.run(args.port).await;
    }
}
