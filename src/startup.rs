use std::net::TcpListener;

use actix_web::dev::Server;
use actix_web::web;
use actix_web::App;
use actix_web::HttpServer;
use sqlx::PgPool;

use crate::routes::health_check;
use crate::routes::subscribe;

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
) -> Result<Server, std::io::Error> {
    // Arc returned.
    let connection = web::Data::new(db_pool);
    // Need to move connection in since the closure will outlive the connection
    // lifetime.
    let server = HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            // Add a count to the arc.
            .app_data(connection.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
