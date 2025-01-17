mod config;
mod handlers;
mod models;
mod errors;
mod repositories;

use crate::config::Config;
use crate::handlers::app_config;
use actix_cors::Cors;
use actix_web::{http::header, http::Method, middleware, App, HttpServer};

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let config = Config::from_env().unwrap();

    let pool = config.configure_pool();
    let hashing_service = config.hashing_service();

    let host = config.server.host;
    let port = config.server.port;
    let server_address = format!("{}:{}", host, port);
    let server_url = config.server.url;

    HttpServer::new(move || {
        let cors = Cors::new()
            .allowed_origin(&server_url)
            .allowed_methods(vec![Method::GET, Method::OPTIONS, Method::POST])
            .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
            .allowed_header(header::CONTENT_TYPE)
            .supports_credentials()
            .finish();

        App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .data(pool.clone())
            .data(hashing_service.clone())
            .configure(app_config)
    })
    .bind(server_address)?
    .run()
    .await
}

#[cfg(test)]
mod integration_tests;
