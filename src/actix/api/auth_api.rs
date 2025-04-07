use actix_jwt_auth_middleware::TokenSigner;
use actix_web::{
    HttpResponse,
    post,
    web::{self},
};
use api::{
    models::user::{LoginRequest, RegisterUser, User, UserJWT},
    schema::users::{self, email, phone_number, table},
};
use diesel::prelude::*;
use jwt_compact::alg::Ed25519;
use validator::Validate;

use crate::DbPool;

#[post("/login")]
async fn login(
    db: web::Data<DbPool>,
    body: web::Json<LoginRequest>,
    token_signer: web::Data<TokenSigner<UserJWT, Ed25519>>,
) -> HttpResponse {
    let mut conn = match db.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("DB connection failed"),
    };

    // Find the user by email
    let result = table
        .filter(email.eq(&body.email))
        .first::<User>(&mut conn)
        .optional();

    let user = match result {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::Unauthorized().body("Invalid email or password"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {e}")),
    };

    // TODO::Compare passwords (plaintext for now — should hash in prod)
    if user.password != body.password {
        return HttpResponse::Unauthorized().body("Invalid email or password");
    }

    let claims = UserJWT { id: user.id };

    // Generate cookies
    let access_cookie = match token_signer.create_access_cookie(&claims) {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Access token error: {e}"));
        }
    };
    let refresh_cookie = match token_signer.create_refresh_cookie(&claims) {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Refresh token error: {e}"));
        }
    };

    HttpResponse::Ok()
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .json(serde_json::json!({
            "message": "Logged in successfully",
            "user": {
                "id": user.id,
                "email": user.email,
                "name": user.name,
                "role": user.role
            }
        }))
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
