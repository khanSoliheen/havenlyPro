use actix_jwt_auth_middleware::TokenSigner;
use actix_web::{
    HttpResponse,
    cookie::SameSite,
    post,
    web::{self},
};
use api::{
    models::user::{LoginRequest, RegisterUser, User, UserJWT},
    schema::users::{self, email, table},
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use diesel::prelude::*;
use jwt_compact::alg::Ed25519;
use password_hash::{SaltString, rand_core::OsRng};
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
        Ok(_) => return HttpResponse::Unauthorized().body("Invalid email or password"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };
    let is_password_correct = if let Ok(parsed_hash) = PasswordHash::new(&user.password) {
        Argon2::default()
            .verify_password(body.password.as_bytes(), &parsed_hash)
            .is_ok()
    } else {
        false // Invalid stored hash format
    };
    if !is_password_correct {
        return HttpResponse::Unauthorized().body("Invalid email or password");
    }

    let claims = UserJWT { id: user.id };

    // Generate cookies
    let mut access_cookie = match token_signer.create_access_cookie(&claims) {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Access token error: {e}"));
        }
    };
    let mut refresh_cookie = match token_signer.create_refresh_cookie(&claims) {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Refresh token error: {e}"));
        }
    };
    // Access token cookie configuration
    access_cookie.set_http_only(true); // ❗ Prevents JavaScript from accessing the cookie (protects against XSS)
    access_cookie.set_secure(false); // ❗ Ensures the cookie is only sent over HTTPS (protects against MITM)
    access_cookie.set_same_site(SameSite::Lax); // ❗ Prevents the cookie from being sent in cross-site requests (protects against CSRF)

    // Refresh token cookie configuration
    refresh_cookie.set_http_only(true);
    refresh_cookie.set_secure(false);
    refresh_cookie.set_same_site(SameSite::Lax);

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

    let mut new_user = user.into_inner();
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hashed_password = argon2
        .hash_password(new_user.password.as_bytes(), &salt)
        .expect("password hashing failed");

    new_user.password = hashed_password.to_string();
    // Insert into database
    match diesel::insert_into(users::table)
        .values(&new_user)
        .execute(&mut *conn)
    {
        Ok(user) => HttpResponse::Created().json(user),
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            err,
        )) => {
            let msg = match err.constraint_name() {
                Some("unique_email") => "email",
                Some("unique_phone") => "phone number",
                Some(c) => c,                // fallback to raw constraint name
                None => "one of the fields", // fallback if not available
            };
            HttpResponse::Conflict().body(format!("User with {} already exists", msg))
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("DB Error: {}", e)),
    }
}

pub fn config_auth_api(cfg: &mut web::ServiceConfig) {
    cfg.service(login).service(register);
}
