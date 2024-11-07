use jt1078_video_server::run_tcp_server;
use jt1078_video_server::server::WebServer;

#[tokio::main]
async fn main() {
    let web_server = WebServer::new("127.0.0.1", 8080).expect("Failed to create web server");
    let tcp_sever_task = run_tcp_server("0.0.0.0", 8000);
    let _ = web_server.run().await;
    tcp_sever_task.end().await;
}
