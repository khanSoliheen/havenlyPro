use std::env;

use actix_cors::Cors;
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, error, middleware::Logger, web};
use collection::operations::validation;
use diesel::{PgConnection, r2d2};
use serde_json::json;
mod actix;

use crate::actix::api::{
    auth_api::config_auth_api,
    health_check_api::configure_health_check,
    users_api::configure_users_api,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if env::var_os("RUST_LOG").is_none() {
        unsafe {
            env::set_var("RUST_LOG", "actix_web=info");
        }
    }
    dotenvy::dotenv().ok();
    env_logger::init();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = r2d2::ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder().build(manager).unwrap();
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(cors)
            .wrap(Logger::default())
    })
    .bind("127.0.0.1:3035")?
    .run()
    .await
}
