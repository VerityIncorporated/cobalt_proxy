pub mod file_deletion_worker;
pub mod routes;

use actix_files::Files;
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use file_deletion_worker::initialize_file_deletion_worker;
use handlebars::Handlebars;
use routes::embed_video;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let file_deletion_queue = initialize_file_deletion_worker();

    let mut handlebars = Handlebars::new();

    handlebars
        .register_template_string("embed", include_str!("../templates/embed.html"))
        .unwrap();

    let file_deletion_queue_ref = web::Data::new(file_deletion_queue);
    let handlebars_ref = web::Data::new(handlebars);

    HttpServer::new(move || {
        App::new()
            .service(
                Files::new("/downloads", "downloads")
                    .show_files_listing()
                    .use_last_modified(true)
                    .prefer_utf8(true),
            )
            .app_data(file_deletion_queue_ref.clone())
            .app_data(handlebars_ref.clone())
            .service(embed_video)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
