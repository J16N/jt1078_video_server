use jt1078_video_server::server::TcpServer;
use jt1078_video_server::server::WebServer;

#[tokio::main]
async fn main() {
    let web_server = WebServer::new("0.0.0.0", 8080).expect("Failed to create web server");
    let mut tcp_server = TcpServer::new("0.0.0.0", 8000);

    let tcp_sever_task = tokio::spawn(async move {
        tcp_server.run().await;
    });
    let _ = web_server.run().await;

    tcp_sever_task.abort();
}
