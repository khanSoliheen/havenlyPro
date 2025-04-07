use chrono::NaiveDateTime;
use diesel::{Insertable, Queryable, Selectable, prelude::QueryableByName};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Serialize, Deserialize)]
pub struct UserJWT {
    pub id: i32,
}

#[derive(Debug, Serialize, Deserialize, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::UserRole"]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    // Admin,
    Customer,
    Professional,
}

#[derive(Debug, Serialize, Deserialize, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::ServiceCategory"]
#[serde(rename_all = "lowercase")]
pub enum ServiceCategory {
    BeautySpa,
    Cleaning,
    Plumbing,
    Carpentry,
    ApplianceRepair,
    Painting,
    HennaArtist,
    Photography,
    Gardening,
    FashionDesign,
    WeddingPlanning,
    EventPlanning,
    WeddingCatering,
    EventCatering,
    WeddingDecor,
    EventDecor,
    WeddingPhoto,
    EventPhoto,
    WeddingVideo,
    EventVideo,
    Other,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, diesel::deserialize::QueryableByName)]
pub struct RawJsonUser {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub user: String,
}

// User model
#[derive(Debug, Serialize, Deserialize, Queryable, Selectable, QueryableByName)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub password: String,
    pub phone_number: String,
    pub professional_info: Option<serde_json::Value>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// Register model
#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Validate)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RegisterUser {
    pub name: String,
    #[validate(email(message = "Please enter a valid email address"))]
    pub email: String,
    pub role: UserRole,
    pub professional_info: Option<serde_json::Value>,
    pub password: String,
    pub phone_number: String,
}

// Insertable user (without ID)
#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub role: UserRole,
    pub professional_info: Option<serde_json::Value>,
}

// Service model
#[derive(Debug, Queryable, Serialize, Insertable)]
#[diesel(table_name = crate::schema::services)]
pub struct Service {
    pub id: i32,
    pub professional_id: i32,
    pub category: ServiceCategory,
    pub description: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    #[diesel(sql_type = schema::services::base_price::SqlType)]
    pub base_price: Decimal,
}
