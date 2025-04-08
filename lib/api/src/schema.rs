// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "booking_status"))]
    pub struct BookingStatus;

    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "service_category"))]
    pub struct ServiceCategory;

    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "user_role"))]
    pub struct UserRole;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::BookingStatus;

    bookings (id) {
        id -> Int4,
        customer_id -> Int4,
        professional_id -> Int4,
        service_id -> Int4,
        scheduled_time -> Timestamp,
        status -> BookingStatus,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ServiceCategory;

    services (id) {
        id -> Int4,
        professional_id -> Int4,
        category -> ServiceCategory,
        #[max_length = 255]
        description -> Nullable<Varchar>,
        base_price -> Numeric,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    transactions (id) {
        id -> Int4,
        booking_id -> Int4,
        amount -> Numeric,
        commission -> Numeric,
        platform_earnings -> Numeric,
        professional_earnings -> Numeric,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::UserRole;

    users (id) {
        id -> Int4,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        role -> UserRole,
        #[max_length = 255]
        password -> Varchar,
        #[max_length = 255]
        phone_number -> Varchar,
        professional_info -> Nullable<Jsonb>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(bookings -> services (service_id));
diesel::joinable!(services -> users (professional_id));
diesel::joinable!(transactions -> bookings (booking_id));

diesel::allow_tables_to_appear_in_same_query!(bookings, services, transactions, users,);
