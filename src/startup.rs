use std::net::TcpListener;

use actix_web::dev::Server;
use actix_web::middleware::Logger;
use actix_web::web;
use actix_web::App;
use actix_web::HttpServer;
use env_logger::Env;
use sqlx::PgPool;

use crate::routes::health_check;
use crate::routes::subscribe;

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
) -> Result<Server, std::io::Error> {
    // Arc returned.
    let connection = web::Data::new(db_pool);
    // `init` does call `set_logger` for us, so this is all we do. We fall back
    // to printing logs at info-level or above if `RUST_LOG` environment
    // variable is not set.
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();
    // Need to move connection in since the closure will outlive the connection
    // lifetime.
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            // Add a count to the arc.
            .app_data(connection.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
