use actix_web::HttpResponse;

/// Simple health-check function that just returns
/// `actix_web::HttpResponse::Ok()`.
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
