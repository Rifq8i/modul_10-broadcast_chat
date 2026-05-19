use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};
use serde::{Deserialize, Serialize};

// Format pesan dari/ke YewChat
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct WebSocketMessage {
    message_type: String,
    data: Option<String>,
    data_array: Option<Vec<String>>,
}

// Format pesan chat yang di-embed dalam data
#[derive(Serialize, Deserialize)]
struct MessageData {
    from: String,
    message: String,
}

type Users = Arc<Mutex<Vec<String>>>;

async fn handle_connection(
    addr: SocketAddr,
    mut ws_stream: WebSocketStream<TcpStream>,
    bcast_tx: broadcast::Sender<String>,
    users: Users,
) {
    let mut username = String::new();
    let mut bcast_rx = bcast_tx.subscribe();

    loop {
        tokio::select! {
            incoming = ws_stream.next() => {
                match incoming {
                    Some(Ok(msg)) => {
                        if let Some(text) = msg.as_text() {
                            // Parse JSON dari YewChat
                            if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(text) {
                                match ws_msg.message_type.as_str() {
                                    "register" => {
                                        // Simpan username
                                        username = ws_msg.data.unwrap_or_default();
                                        println!("User registered: {username} from {addr}");

                                        // Tambah ke daftar user
                                        users.lock().await.push(username.clone());

                                        // Broadcast daftar user terbaru ke semua client
                                        let user_list = users.lock().await.clone();
                                        let response = WebSocketMessage {
                                            message_type: "users".to_string(),
                                            data: None,
                                            data_array: Some(user_list),
                                        };
                                        let _ = bcast_tx.send(
                                            serde_json::to_string(&response).unwrap()
                                        );
                                    }
                                    "message" => {
                                        let content = ws_msg.data.unwrap_or_default();
                                        println!("Message from {username}: {content}");

                                        // Bungkus pesan dengan info pengirim
                                        let message_data = MessageData {
                                            from: username.clone(),
                                            message: content,
                                        };

                                        // Kirim sebagai WebSocketMessage type "message"
                                        let response = WebSocketMessage {
                                            message_type: "message".to_string(),
                                            data: Some(
                                                serde_json::to_string(&message_data).unwrap()
                                            ),
                                            data_array: None,
                                        };
                                        let _ = bcast_tx.send(
                                            serde_json::to_string(&response).unwrap()
                                        );
                                    }
                                    _ => {
                                        println!("Unknown message type from {addr}");
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        println!("Client {addr} ({username}) disconnected");

                        // Hapus dari daftar user
                        if !username.is_empty() {
                            users.lock().await.retain(|u| u != &username);

                            // Broadcast daftar user terbaru
                            let user_list = users.lock().await.clone();
                            let response = WebSocketMessage {
                                message_type: "users".to_string(),
                                data: None,
                                data_array: Some(user_list),
                            };
                            let _ = bcast_tx.send(
                                serde_json::to_string(&response).unwrap()
                            );
                        }
                        break;
                    }
                }
            }
            msg = bcast_rx.recv() => {
                if let Ok(text) = msg {
                    let _ = ws_stream.send(Message::text(text)).await;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (bcast_tx, _) = broadcast::channel(16);
    let users: Users = Arc::new(Mutex::new(Vec::new()));

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Listening on port 8080");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New connection from {addr}");
        let bcast_tx = bcast_tx.clone();
        let users = users.clone();

        tokio::spawn(async move {
            let ws_stream = ServerBuilder::new().accept(socket).await.unwrap();
            handle_connection(addr, ws_stream, bcast_tx, users).await;
        });
    }
}