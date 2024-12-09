use std::{env, path::Path};

use actix_web::{get, http::header, web, HttpRequest, HttpResponse, Responder};
use chrono::Utc;
use handlebars::Handlebars;
use tokio::{fs::File, io::AsyncWriteExt};
use url::Url;
use uuid::Uuid;

use crate::file_deletion_worker::FileDeletionQueue;

#[get("/{video_url:.*}")]
pub async fn embed_video(
    req: HttpRequest,
    handle_bars: web::Data<Handlebars<'_>>,
    file_deletion_queue: web::Data<FileDeletionQueue>,
) -> impl Responder {
    let instance_uri =
        env::var("INSTANCE_URI").expect("Expected INSTANCE_URI in environment variables.");
    let mut video_url = req.uri().to_string();
    video_url.remove(0);

    println!("Hello: {}", video_url);

    if let (Ok(instance_url), Ok(video_url)) = (Url::parse(&instance_uri), Url::parse(&video_url)) {
        if instance_url.host() != video_url.host() {
            return HttpResponse::Forbidden()
                .body("The requested video URL is not from the same instance.");
        }
    } else {
        return HttpResponse::BadRequest().body("Invalid URL provided.");
    }

    let uuid = Uuid::new_v4().to_string();
    let response = reqwest::get(&video_url).await;

    match response {
        Ok(ok_response) => {

            let mut filename = uuid.clone();

            if let Some(content_disposition) = ok_response.headers().get("Content-Disposition") {
                if let Ok(content_disposition_str) = content_disposition.to_str() {
                    if let Some(filename_from_header) = content_disposition_str.split("filename=").nth(1) {
                        filename = filename_from_header.trim_matches('"').to_string();
                    }
                }
            }

            let file_path = format!("downloads/{}", filename);

            println!("File path: {}", file_path);

            match ok_response.bytes().await {
                Ok(file_bytes) => {
                    let mut created_file = match File::create(&file_path).await {
                        Ok(file) => file,
                        Err(err) => {
                            eprintln!("Error creating file: {}", err);
                            return HttpResponse::InternalServerError().body("Failed to create file.");
                        }
                    };

                    if let Err(err) = created_file.write_all(&file_bytes).await {
                        eprintln!("Error writing to file: {}", err);
                        return HttpResponse::InternalServerError().body("Failed to write video to file.");
                    }

                    let extension = Path::new(&filename).extension().and_then(|ext| ext.to_str()).unwrap_or("");
                    let is_video = match extension.to_lowercase().as_str() {
                        "mp4" | "webm" | "avi" | "mov" => true,
                        _ => false,
                    };
                    
                    let mut context = std::collections::HashMap::new();
                    context.insert("filename", filename.clone());
                    context.insert("url", format!("https://{}/downloads/{}", req.headers().get(header::HOST).unwrap().to_str().unwrap(), filename));
                    context.insert("is_video", is_video.to_string());
                    context.insert("error_message", String::new());

                    let rendered_html = handle_bars.render("embed", &context).unwrap_or_else(|_| {
                        eprintln!("Error rendering template.");
                        "<h1>Error rendering the page</h1>".to_string()
                    });

                    let deletion_time = Utc::now() + chrono::Duration::minutes(25);
                    let mut queue = file_deletion_queue.lock().await;
                    queue.push_back((file_path, deletion_time));

                    HttpResponse::Ok().content_type("text/html").body(rendered_html)
                }
                Err(err) => {
                    eprintln!("Error downloading file: {}", err);
                    HttpResponse::InternalServerError().body("Failed to download video.")
                }
            }
        }
        Err(err) => {
            eprintln!("Error sending request: {:?}", err);
            HttpResponse::BadRequest().body(format!("Error: {:?}", err))
        }
    }
}