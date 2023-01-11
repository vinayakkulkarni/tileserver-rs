use actix_web::{App, HttpServer};

mod routes;
mod drivers;

mod structs;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(routes::routes::health)
            .service(routes::routes::index_json)
            .service(routes::routes::styles_json)
            .service(routes::routes::rendered_json)
            .service(routes::routes::data_json)
    })
    .bind(("127.0.0.1", 3000))?
    .run()
    .await
}
