use actix_web::{
    HttpResponse,
    post,
    web::{self},
};
use api::{models::user::RegisterUser, schema::users};
use diesel::prelude::*;
use validator::Validate;

use crate::DbPool;

#[post("/login")]
async fn login() -> HttpResponse {
    HttpResponse::Ok().body("login")
}

#[post("/register")]
async fn register(pool: web::Data<DbPool>, user: web::Json<RegisterUser>) -> HttpResponse {
    // Validate user input
    match user.validate() {
        Ok(_) => (),
        Err(err) => return HttpResponse::BadRequest().json(err),
    };

    // Get a database connection
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to get DB connection"),
    };

    let new_user = user.into_inner();

    // Insert into database
    match diesel::insert_into(users::table)
        .values(&new_user)
        .execute(&mut *conn)
    {
        Ok(_) => HttpResponse::Created().json(new_user),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB Error: {}", e)),
    }
}

pub fn config_auth_api(cfg: &mut web::ServiceConfig) {
    cfg.service(login).service(register);
}
