use actix_web::{
    HttpResponse,
    post,
    web::{self},
};
use api::{
    models::user::RegisterUser,
    schema::users::{self, email, phone_number},
};
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
        .on_conflict((email, phone_number))
        .do_nothing()
        .execute(&mut *conn)
    {
        Ok(_) => HttpResponse::Created().json(new_user),
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        )) => HttpResponse::Conflict()
            .body(format!("User with email {} already exists", new_user.email)),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB Error: {}", e)),
    }
}

pub fn config_auth_api(cfg: &mut web::ServiceConfig) {
    cfg.service(login).service(register);
}
