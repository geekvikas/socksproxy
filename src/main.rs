#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))]
#![warn(clippy::all, rust_2018_idioms)]
#[macro_use]
extern crate log;
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use futures::*;
use socksproxy::*;
use std::env;
use std::error::Error;

#[derive(Parser, Debug)]
#[clap(version)]
struct Opt {
    #[clap(short, long, default_value_t = 1080)]
    /// Set port to listen on
    port: u16,

    #[clap(short, long, default_value = "0.0.0.0")]
    /// Set ip to listen on
    ip: String,

    /// Do not output any logs (even errors!). Overrides `RUST_LOG`
    #[clap(short)]
    quiet: bool,
}

#[get("/health")]
async fn echo(_req_body: String) -> impl Responder {
    HttpResponse::Ok()
        .append_header(("Content-Type", "application/json"))
        .body("{status:\"ok\"}")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    info!("Starting SOCKS Proxy Server");

    let opt = Opt::parse();

    // Setup logging
    let log_env = env::var("RUST_LOG");
    if log_env.is_err() {
        let level = "socksproxy=INFO";
        info!("Logging is set to {}", level);
        env::set_var("RUST_LOG", level);
    }

    if !opt.quiet {
        pretty_env_logger::init_timed();
    }

    // Setup Proxy settings

    let mut auth_methods: Vec<u8> = Vec::new();
    auth_methods.push(socksproxy::AuthMethods::NoAuth as u8);

    // Create proxy server
    let mut socksproxy = SocksProxy::new(opt.port, &opt.ip, auth_methods, None).await?;
    info!("Listening on {}:{}", opt.ip, opt.port);

    let http = HttpServer::new(|| App::new().service(echo))
        .bind(("0.0.0.0", 8080))?
        .run();

    let _ok = future::join(http, socksproxy.serve()).await;

    Ok(())
}
