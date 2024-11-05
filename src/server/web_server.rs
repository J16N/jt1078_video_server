use std::{net::SocketAddr, path::PathBuf};

use crate::Result;
use actix_files::NamedFile;
use actix_web::{dev::Server, get, web, App, HttpResponse, HttpServer, Responder};

#[get("/health_check")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[derive(serde::Deserialize)]
struct Stream {
    imei: String,
    segment: String,
}

#[get("/{imei}/{segment}.ts")]
async fn get_segment(stream: web::Path<Stream>) -> impl Responder {
    let path = PathBuf::from(format!("{}/streams/{}.ts", stream.imei, stream.segment));
    if path.exists() {
        let content = match tokio::fs::read(path).await {
            Ok(content) => content,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };
        HttpResponse::Ok().content_type("video/mp2t").body(content)
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[get("/{imei}/playlist.m3u8")]
async fn get_playlist(imei: web::Path<String>) -> impl Responder {
    let path = PathBuf::from(format!("{}/playlist.m3u8", imei));
    NamedFile::open_async(path).await
}

pub struct WebServer {
    address: SocketAddr,
    server: Server,
}

impl WebServer {
    pub async fn run(self) -> std::io::Result<()> {
        println!("HTTP Server listening on {}", self.address);
        self.server.await
    }

    pub fn new(host: &str, port: u16) -> Result<Self> {
        let port: u16 = std::env::var("HTTP_PORT")
            .unwrap_or_else(|_| port.to_string())
            .parse()
            .expect("Failed to parse port");

        let address: SocketAddr = format!("{}:{}", host, port)
            .parse()
            .expect("Failed to parse address");

        let listener = std::net::TcpListener::bind(address).expect("Failed to bind to address");

        let address = listener.local_addr().expect("Failed to get local address");

        let server = HttpServer::new(move || {
            App::new().service(health_check).service(
                web::scope("/streams")
                    .service(get_segment)
                    .service(get_playlist),
            )
        })
        .listen(listener)?
        .run();

        Ok(Self { address, server })
    }
}
