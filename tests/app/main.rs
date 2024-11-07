mod tcp_client;

use jt1078_video_server::server::WebServer;
use jt1078_video_server::{run_tcp_server, TcpServerTask};
use once_cell::sync::Lazy;
use std::net::SocketAddr;
use std::sync::LazyLock;
use tcp_client::TcpClient;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;

struct MyTasks {
    client_task: Option<JoinHandle<()>>,
    tcp_server_task: Option<TcpServerTask>,
    web_server_task: JoinHandle<std::io::Result<()>>,
}

impl MyTasks {
    fn new(
        client: JoinHandle<()>,
        tcp_server_task: TcpServerTask,
        web_server_task: JoinHandle<std::io::Result<()>>,
    ) -> Self {
        Self {
            client_task: Some(client),
            tcp_server_task: Some(tcp_server_task),
            web_server_task,
        }
    }
}

struct MyTests {
    sender: broadcast::Sender<()>,
}

impl MyTests {
    async fn increment(&self) {
        let mut receiver = self.sender.subscribe();
        let _ = receiver.recv().await;
        let mut ntests = unsafe { NTESTS.lock().await };
        *ntests += 1;
    }

    async fn decrement(&self) {
        let mut ntests = unsafe { NTESTS.lock().await };
        *ntests -= 1;

        if *ntests == 0 {
            let mut my_tasks = unsafe { TASKS.take().unwrap() };

            let client_task = my_tasks.client_task.take().unwrap();
            if let Err(e) = client_task.await {
                eprintln!("Failed to wait for client task: {}", e);
            }

            let tcp_server_task = my_tasks.tcp_server_task.take().unwrap();
            tcp_server_task.end().await;

            my_tasks.web_server_task.abort();
        }
    }
}

static mut NTESTS: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));

static mut TASKS: Option<MyTasks> = None;

static TESTS: Lazy<MyTests> = Lazy::new(|| {
    let host = "127.0.0.1";
    let port = 8000;
    let address: SocketAddr = format!("{}:{}", host, port).parse().unwrap();

    let tcp_server_task = run_tcp_server(host, port);
    let web_server = WebServer::new("127.0.0.1", 8080).expect("Failed to create web server");
    let web_server_task = tokio::spawn(web_server.run());
    let mut client = TcpClient::new(address, "data/test_stream.bin");

    let (tx, _) = broadcast::channel(1);
    let atx = tx.clone();

    let client_task = tokio::spawn(async move {
        client.connect().await.expect("Failed to connect to server");
        match client.send().await {
            Ok(_) => {
                println!("Data sent successfully");
                let _ = atx.send(());
            }
            Err(e) => eprintln!("Failed to send data: {}", e),
        };
        client.close().await.expect("Failed to close connection");
    });

    unsafe {
        TASKS = Some(MyTasks::new(client_task, tcp_server_task, web_server_task));
    }
    MyTests { sender: tx }
});

async fn get_playlist_content() -> String {
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8080/streams/353071279375/playlist.m3u8")
        .send()
        .await
        .unwrap();
    response.text().await.unwrap()
}

#[tokio::test]
async fn test_health_check() {
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8080/health_check")
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());
}

#[tokio::test]
async fn test_get_playlist() {
    TESTS.increment().await;

    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8080/streams/353071279375/playlist.m3u8")
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "audio/x-mpegurl"
    );

    TESTS.decrement().await;
}

#[tokio::test]
async fn test_get_segment() {
    TESTS.increment().await;

    let content = get_playlist_content().await;
    let segment = content
        .lines()
        .rev()
        .find(|line| line.ends_with(".ts"))
        .unwrap();

    let url = format!("http://127.0.0.1:8080/streams/353071279375/{segment}");

    let client = reqwest::Client::new();
    let response = client.get(url).send().await.unwrap();

    assert!(response.status().is_success());
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "video/mp2t"
    );

    TESTS.decrement().await;
}

#[tokio::test]
async fn test_total_segments() {
    TESTS.increment().await;

    let content = get_playlist_content().await;
    let segments = content.lines().filter(|line| line.ends_with(".ts")).count();

    assert_eq!(segments, 10);

    TESTS.decrement().await;
}

#[tokio::test]
async fn test_segment_duration() {
    TESTS.increment().await;

    let content = get_playlist_content().await;
    let duration: f64 = content
        .lines()
        .find(|line| line.starts_with("#EXT-X-TARGETDURATION"))
        .unwrap()
        .split(':')
        .last()
        .unwrap()
        .parse()
        .unwrap();

    assert!((6.0..7.0).contains(&duration));

    TESTS.decrement().await;
}
