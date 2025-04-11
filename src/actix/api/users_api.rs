use actix_web::{
    HttpResponse,
    cookie::{Cookie, time::Duration},
    get,
    web::{self, Data, Path, Query},
};
use api::models::{common::FieldSelection, user::RawJsonUser};
use diesel::{
    prelude::*,
    sql_types::{Integer, Text},
};
use serde_json::{Value, from_str, json};

use crate::DbPool;

#[get("")]
async fn get_users(pool: web::Data<DbPool>, input: web::Query<FieldSelection>) -> HttpResponse {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to get DB connection: {}", err));
        }
    };

    let allowed_fields = [
        "id",
        "name",
        "email",
        "role",
        "professional_info",
        "phone_number",
        "created_at",
        "updated_at",
    ];

    let selected_fields: Vec<&str> = match &input.fields {
        Some(fields) => fields
            .split(',')
            .map(|f| f.trim())
            .filter(|f| allowed_fields.contains(f))
            .collect(),
        None => allowed_fields.to_vec(),
    };

    if selected_fields.is_empty() {
        return HttpResponse::BadRequest().body("No valid fields provided.");
    }

    // Build dynamic WHERE clauses and params
    let mut filters = vec![];
    let mut bind_values: Vec<String> = vec![];
    let mut param_counter = 1;

    if let Some(email) = &input.email {
        filters.push(format!("email ILIKE ${}", param_counter));
        bind_values.push(format!("%{}%", email));
        param_counter += 1;
    }

    if let Some(name) = &input.name {
        filters.push(format!("name ILIKE ${}", param_counter));
        bind_values.push(format!("%{}%", name));
        param_counter += 1;
    }

    if let Some(role) = &input.role {
        filters.push(format!("role = ${}", param_counter));
        bind_values.push(role.clone());
        param_counter += 1;
    }

    if let Some(phone) = &input.phone_number {
        filters.push(format!("phone_number ILIKE ${}", param_counter));
        bind_values.push(format!("%{}%", phone));
    }

    let where_clause = if !filters.is_empty() {
        format!("WHERE {}", filters.join(" AND "))
    } else {
        String::new()
    };

    let sort_field = input
        .sort_by
        .as_deref()
        .filter(|f| allowed_fields.contains(f))
        .unwrap_or("id");

    let sort_order = input
        .order
        .as_deref()
        .map(|o| o.to_uppercase())
        .filter(|o| o == "ASC" || o == "DESC")
        .unwrap_or_else(|| "DESC".to_string());

    let sql = format!(
        "SELECT row_to_json(u) as user FROM (
            SELECT {} FROM users {} ORDER BY {} {} LIMIT ${} OFFSET ${}
        ) u",
        selected_fields.join(", "),
        where_clause,      // e.g. "WHERE email ILIKE $1"
        sort_field,        // validated & safe
        sort_order,        // e.g. "DESC" or "ASC"
        param_counter,     // next bind: LIMIT
        param_counter + 1  // next bind: OFFSET
    );

    // Create boxed query to handle dynamic bind count
    let mut query = diesel::sql_query(sql).into_boxed();
    for value in bind_values {
        query = query.bind::<Text, _>(value);
    }

    query = query
        .bind::<Integer, _>(input.limit.unwrap_or(10))
        .bind::<Integer, _>(input.off_set.unwrap_or(0));

    // Execute with boxed query
    let result: Result<Vec<RawJsonUser>, _> = query.get_results(&mut *conn);

    match result {
        Ok(rows) => {
            let parsed: Result<Vec<Value>, _> = rows
                .into_iter()
                .map(|r| serde_json::from_str::<Value>(&r.user))
                .collect();

            match parsed {
                Ok(users) => HttpResponse::Ok().json(json!({ "users": users })),
                Err(e) => {
                    HttpResponse::InternalServerError().body(format!("JSON parse error: {e}"))
                }
            }
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {:?}", e)),
    }
}

#[get("/{user_id}")]
async fn get_user(
    pool: Data<DbPool>,
    user_id: Path<i32>,
    query: Query<FieldSelection>,
) -> HttpResponse {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to get DB connection with error: {}", err));
        }
    };
    let uid = user_id.into_inner();

    // Whitelisted fields (safe to expose)
    let allowed_fields = [
        "id",
        "name",
        "email",
        "role",
        "professional_info",
        "phone_number",
        "created_at",
        "updated_at",
    ];

    // Determine selected fields
    let selected_fields: Vec<&str> = match &query.fields {
        Some(fields) => fields
            .split(',')
            .map(|f| f.trim())
            .filter(|f| allowed_fields.contains(f))
            .collect(),
        None => allowed_fields.to_vec(),
    };

    if selected_fields.is_empty() {
        return HttpResponse::BadRequest().body("No valid fields provided.");
    }

    let sql = format!(
        "SELECT row_to_json(u) as user FROM (SELECT {} FROM users WHERE id = $1) u",
        selected_fields.join(", ")
    );

    let result: Result<RawJsonUser, _> = diesel::sql_query(sql)
        .bind::<Integer, _>(uid)
        .get_result(&mut *conn);

    match result {
        Ok(raw) => match from_str::<Value>(&raw.user) {
            Ok(user) => HttpResponse::Ok().json(json!({"user": user})),
            Err(_) => HttpResponse::InternalServerError().body("Failed to parse JSON"),
        },
        Err(diesel::result::Error::NotFound) => {
            HttpResponse::NotFound().body(format!("User not found with the provided id {}", uid))
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {:?}", e)),
    }
}

#[get("/logout")]
async fn logout() -> HttpResponse {
    let clear_access = Cookie::build("access_token", "")
        .http_only(true)
        .secure(true)
        .max_age(Duration::seconds(0))
        .finish();

    let clear_refresh = Cookie::build("refresh_token", "")
        .http_only(true)
        .secure(true)
        .max_age(Duration::seconds(0))
        .finish();

    HttpResponse::Ok()
        .cookie(clear_access)
        .cookie(clear_refresh)
        .json(serde_json::json!({ "message": "Logged out successfully" }))
}

pub fn configure_users_api(cfg: &mut web::ServiceConfig) {
    cfg.service(get_users);
    cfg.service(get_user);
    cfg.service(logout);
}
