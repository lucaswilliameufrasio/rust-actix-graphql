mod graphql;

use actix_web::{web, HttpResponse};
use deadpool_postgres::Pool;
use graphql::{create_schema, Context, Schema};
use juniper::http::{graphiql::graphiql_source, GraphQLRequest};
use std::sync::Arc;
use crate::{config::HashingService, repositories::post::get_posts_loader};

async fn health() -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub fn app_config(config: &mut web::ServiceConfig) {
    let schema = create_schema();
    config
        .data(schema)
        .service(web::resource("/").route(web::get().to(health)))
        .service(web::resource("/graphql").route(web::post().to(graphql)))
        .service(web::resource("/graphiql").route(web::get().to(graphiql)));
}

async fn graphiql() -> HttpResponse {
    let html = graphiql_source("/graphql");

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

async fn graphql(
    data: web::Json<GraphQLRequest>,
    schema: web::Data<Schema>,
    pool: web::Data<Pool>,
    hashing_service: web::Data<HashingService>
) -> HttpResponse {
    let pool: Arc<Pool> = pool.into_inner();
    let hashing = hashing_service.into_inner();
    let post_loader = get_posts_loader(pool.clone());
    let context: Context = Context { pool, hashing, post_loader };

    let res = data.execute(&schema, &context).await;

    HttpResponse::Ok().json(res)
}
