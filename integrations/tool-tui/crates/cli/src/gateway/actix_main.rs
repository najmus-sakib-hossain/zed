//! Main Actix-web server

use super::{middleware, rest_api};
use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger};
use anyhow::Result;

pub async fn start_server(host: &str, port: u16) -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Starting DX Gateway Server at http://{}:{}", host, port);

    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .wrap(middleware::Auth)
            .configure(rest_api::configure)
    })
    .bind((host, port))?
    .run()
    .await?;

    Ok(())
}
