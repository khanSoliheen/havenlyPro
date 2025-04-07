use actix_web::{HttpResponse, get, web};

#[get("/health_check")]
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json("health_check")
}

pub fn configure_health_check_api(cfg: &mut web::ServiceConfig) {
    cfg.service(health_check);
}
