use std::env;

use actix::api::health_check_api::configure_health_check_api;
use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_jwt_auth_middleware::{Authority, TokenSigner, use_jwt::UseJWTOnApp};
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, error, middleware::Logger, web};
use api::models::user::UserJWT;
use collection::{
    connections::connections::{create_amqp_channel, create_redis_conn},
    operations::validation,
};
use diesel::{
    PgConnection,
    r2d2::{self, ConnectionManager},
};
use ed25519_compact::{KeyPair, Seed};
use jwt_compact::alg::Ed25519;
use serde_json::json;
mod actix;

use crate::actix::api::{auth_api::config_auth_api, users_api::configure_users_api};

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if env::var_os("RUST_LOG").is_none() {
        unsafe {
            env::set_var("RUST_LOG", "actix_web=info");
        }
    }
    dotenvy::dotenv().ok();
    env_logger::init();
    create_redis_conn().await;
    create_amqp_channel().await;
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = r2d2::ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder().build(manager).unwrap();
    let key_pair = KeyPair::from_seed(Seed::default());
    HttpServer::new(move || {
        let governor_conf = GovernorConfigBuilder::default()
            .requests_per_second(1) // 1 request per second
            .burst_size(5) // allow up to 5 bursts at once
            .finish()
            .unwrap();
        let governor = Governor::new(&governor_conf);
        let signer = TokenSigner::<UserJWT, Ed25519>::new()
            .signing_key(key_pair.sk.clone())
            .algorithm(Ed25519)
            .build()
            .expect("token signer build failed");

        let authority = Authority::new()
            .refresh_authorizer(|| async move { Ok(()) })
            .token_signer(Some(signer))
            .verifying_key(key_pair.sk.public_key())
            .build()
            .expect("authority build failed");

        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
        let validate_path_config = actix_web_validator::PathConfig::default()
            .error_handler(|err, rec| validation_error_handler("path", err, rec));
        let validate_query_config = actix_web_validator::QueryConfig::default()
            .error_handler(|err, rec| validation_error_handler("query", err, rec));
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(validate_path_config)
            .app_data(validate_query_config)
            .wrap(governor)
            .wrap(cors)
            .wrap(Logger::default())
            .configure(configure_health_check_api)
            .configure(config_auth_api)
            .use_jwt(
                authority,
                web::scope("/users").configure(configure_users_api),
            )
    })
    .bind("127.0.0.1:3035")?
    .run()
    .await
}

fn validation_error_handler(
    name: &str,
    err: actix_web_validator::Error,
    _req: &HttpRequest,
) -> error::Error {
    use actix_web_validator::error::DeserializeErrors;

    // Nicely describe deserialization and validation errors
    let msg = match &err {
        actix_web_validator::Error::Validate(errs) => {
            validation::label_errors(format!("Validation error in {name}"), errs)
        }
        actix_web_validator::Error::Deserialize(err) => {
            format!(
                "Deserialize error in {name}: {}",
                match err {
                    DeserializeErrors::DeserializeQuery(err) => err.to_string(),
                    DeserializeErrors::DeserializeJson(err) => err.to_string(),
                    DeserializeErrors::DeserializePath(err) => err.to_string(),
                }
            )
        }
        actix_web_validator::Error::JsonPayloadError(
            actix_web::error::JsonPayloadError::Deserialize(err),
        ) => {
            format!("Format error in {name}: {err}",)
        }
        err => err.to_string(),
    };

    // Build fitting response
    let response = match &err {
        actix_web_validator::Error::Validate(_) => HttpResponse::UnprocessableEntity(),
        _ => HttpResponse::BadRequest(),
    }
    .json(json!({ "error": msg }));
    error::InternalError::from_response(err, response).into()
}
