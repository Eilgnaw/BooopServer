// #![deny(warnings)]
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use serde::Deserialize;
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;
use tokio_tungstenite::connect_async;
use std::net::SocketAddr;
/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

#[macro_use]
mod utils;

/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`
// type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;


type Users = Arc<RwLock<HashMap<usize, (Option<String>, mpsc::UnboundedSender<Message>)>>>;

#[derive(Deserialize)]
struct LoginMessage {
    cmd: String,
    id: Option<String>,
}

#[derive(Deserialize)]
struct SendMessage {
    cmd: String,
    to: Option<String>,  // 目标用户 UUID
    text: Option<String>, // 消息内容
}

#[derive(Deserialize)]
struct HSendMessage {
    to: String,  
    emoji: String, 
}



#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // Keep track of all connected users, key is usize, value
    // is a websocket sender.
    let users = Users::default();
    // Turn our "state" into a new Filter...
    let users = warp::any().map(move || users.clone());

    // GET /chat -> websocket upgrade
    let chat = warp::path("chat")
        // The `ws()` filter will prepare Websocket handshake...
        .and(warp::ws())
        .and(users)
        .map(|ws: warp::ws::Ws, users| {
            // This will call our function if the handshake succeeds.
            ws.on_upgrade(move |socket| user_connected(socket, users))
        });


    // let send_message_route = warp::path!("send_message" / String)
    //     .and(warp::post())
    //     .and(warp::body::json())
    //     .and(warp::any().map(move || users.clone()))
    //     .and_then(send_message_to_user);

        let send_message_route = warp::path("send_message")
            .and(warp::addr::remote())
            .and(warp::post())
            .and(warp::body::json())
            .and_then(send_message_to_user);

    // GET / -> index html
    // let index = warp::path::end().map(|| warp::reply::html(INDEX_HTML));

    let routes = send_message_route.or(chat);
    //.or(send_message);

    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}

async fn send_message_to_user(
    addr: Option<SocketAddr>,
    body: HSendMessage,
    
) -> Result<impl warp::Reply, warp::Rejection> {

    match addr {
        Some(address) => {
            eprintln_with_time!("Received request from IP: {} and Port: {}", address.ip(), address.port());
        },
        None => {
            eprintln_with_time!("No remote address found");
        }
    }

    let ws_url = "ws://127.0.0.1:3030/chat";

    // Try to establish a WebSocket connection
    let (ws_stream, _) = match connect_async(ws_url).await {
        Ok(connection) => connection,
        Err(e) => {
            eprintln_with_time!("Failed to connect to WebSocket server: {}", e);
            // return Ok(warp::reply::with_status(
            //     "Failed to connect to WebSocket server",
            //     StatusCode::INTERNAL_SERVER_ERROR,
            // ));
            return Ok(warp::reply::with_status( "Failed to connect to WebSocket server", warp::http::StatusCode::INTERNAL_SERVER_ERROR))
        }
    };

    // Extract the message and user UUID from the request body
    let HSendMessage {  to, emoji } = body;

    // Construct the message to be sent via WebSocket
    let ws_message = format!(
        "{{\"cmd\": \"send\", \"to\": \"{}\", \"text\": \"{}\"}}",
        to, emoji
    );

    // Send the message over the WebSocket
    let (mut write, _read) = ws_stream.split();
    if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Text(ws_message)).await {
        eprintln_with_time!("Failed to send message: {}", e);
       
        return Ok(warp::reply::with_status( "Failed to send message over WebSocket", warp::http::StatusCode::INTERNAL_SERVER_ERROR))
    }
     Ok(warp::reply::with_status("Message sent successfully", warp::http::StatusCode::OK))
}

// async fn send_message_to_user(
//     users: Users,
//     user_uuid: String,
//     message: String,
// ) -> Result<impl warp::Reply, warp::Rejection> {
//     return Ok(warp::reply::with_status("User not found", warp::http::StatusCode::NOT_FOUND));
//     // let users_read = users.read().await;

//     // if let Some(tx) = users_read.get(&user_uuid) {
//     //     if tx.send(Message::text(message)).is_err() {
//     //         return Ok(warp::reply::with_status(
//     //             "Failed to send message",
//     //             warp::http::StatusCode::INTERNAL_SERVER_ERROR,
//     //         ));
//     //     }
//     //     Ok(warp::reply::with_status("Message sent", warp::http::StatusCode::OK))
//     // } else {
//     //     Ok(warp::reply::with_status(
//     //         "User not connected",
//     //         warp::http::StatusCode::NOT_FOUND,
//     //     ))
//     // }
// }


// async fn send_message_to_user(users: &Users, user_uuid: String, message: String) -> Result<impl warp::Reply, warp::Rejection> {
//     // let users_read = users.read().await;
//     // let new_msg = format!("{}", message);
//     // if let Some((_, tx)) = users.iter().find(|(_, (uuid, _))| uuid.as_ref() == Some(&user_uuid)) {
//     //     if let Err(_disconnected) = tx.1.send(Message::text(new_msg.clone())) {
//     //         eprintln_with_time!("User with UUID {} is disconnected", user_uuid);
//     //         Ok(warp::reply::with_status("User is disconnected", warp::http::StatusCode::NOT_FOUND));
//     //     }
//     //     Ok(warp::reply::with_status("Message sent", warp::http::StatusCode::OK));
//     // } else {
//     //     eprintln_with_time!("User with UUID {} not found", user_uuid);
//     //     Ok(warp::reply::with_status("User not found", warp::http::StatusCode::NOT_FOUND));
//     // }

//     return Ok(warp::reply::with_status("User not found", warp::http::StatusCode::NOT_FOUND));

//     // if let Some(tx) = users_read.get(&user_uuid) {
//     //     if tx.send(Message::text(message)).is_err() {
//     //         return Ok(warp::reply::with_status("Failed to send message", warp::http::StatusCode::INTERNAL_SERVER_ERROR));
//     //     }
//     //     Ok(warp::reply::with_status("Message sent", warp::http::StatusCode::OK))
//     // } else {
//     //     Ok(warp::reply::with_status("User not connected", warp::http::StatusCode::NOT_FOUND))
//     // }
// }


async fn user_connected(ws: WebSocket, users: Users) {
    // Use a counter to assign a new unique ID for this user.
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    eprintln_with_time!("new chat user: {}", my_id);

    // Split the socket into a sender and receive of messages.
    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            user_ws_tx
                .send(message)
                .unwrap_or_else(|e| {
                    eprintln_with_time!("websocket send error: {}", e);
                })
                .await;
        }
    });

    // Save the sender in our list of connected users.
    users.write().await.insert(my_id, (None, tx));

    // Return a `Future` that is basically a state machine managing
    // this specific user's connection.

    // Every time the user sends a message, broadcast it to
    // all other users...
    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln_with_time!("websocket error(uid={}): {}", my_id, e);
                break;
            }
        };
        user_message(my_id, msg, &users).await;
    }

    // user_ws_rx stream will keep processing as long as the user stays
    // connected. Once they disconnect, then...
    user_disconnected(my_id, &users).await;
}

async fn user_message(my_id: usize, msg: Message, users: &Users) {
    // Skip any non-Text messages...
    let msg = if let Ok(s) = msg.to_str() {
        s
    } else {
        return;
    };
    eprintln_with_time!("msg{}",msg);
    if let Ok(parsed) = serde_json::from_str::<LoginMessage>(msg) {
        if parsed.cmd == "login" {
            if let Some(new_id) = parsed.id {
                // 将用户 UUID 存储到对应的连接
                let mut users = users.write().await;
                if let Some((uuid, _)) = users.get_mut(&my_id) {
                    *uuid = Some(new_id.clone());
                }
                // eprintln_with_time!("new chat user: {}", my_id);
                eprintln_with_time!("User#{} logged in as {}", my_id, new_id);
            }
            return;
        }
    }


    if let Ok(parsed) = serde_json::from_str::<SendMessage>(msg) {
        if parsed.cmd == "send" {
            if let (Some(to), Some(text)) = (parsed.to, parsed.text) {
                let new_msg = format!("{}", text);

                // 找到目标用户 UUID 对应的连接并发送消息
                let users = users.read().await;
                if let Some((_, tx)) = users.iter().find(|(_, (uuid, _))| uuid.as_ref() == Some(&to)) {
                    if let Err(_disconnected) = tx.1.send(Message::text(new_msg.clone())) {
                        eprintln_with_time!("User with UUID {} is disconnected", to);
                    }
                } else {
                    eprintln_with_time!("User with UUID {} not found", to);
                }
            }
            return;
        }
    }

    eprintln_with_time!("Received invalid message: {}", msg);

    // let new_msg = format!("<User#{}>: {}", my_id, msg);

    // // New message from this user, send it to everyone else (except same uid)...
    // for (&uid, (uuid, tx)) in users.read().await.iter() {
    //     if my_id != uid {
    //         if let Err(_disconnected) = tx.send(Message::text(new_msg.clone())) {
    //             // The tx is disconnected, our `user_disconnected` code
    //             // should be happening in another task, nothing more to
    //             // do here.
    //         }
    //     }
    // }
}

async fn user_disconnected(my_id: usize, users: &Users) {
    eprintln_with_time!("good bye user: {}", my_id);

    // Stream closed up, so remove from the user list
    users.write().await.remove(&my_id);
}

// static INDEX_HTML: &str = r#"<!DOCTYPE html>
// <html lang="en">
//     <head>
//         <title>Warp Chat</title>
//     </head>
//     <body>
//         <h1>Warp chat</h1>
//         <div id="chat">
//             <p><em>Connecting...</em></p>
//         </div>
//         <input type="text" id="text" />
//         <button type="button" id="send">Send</button>
//         <script type="text/javascript">
//         const chat = document.getElementById('chat');
//         const text = document.getElementById('text');
//         const uri = 'ws://' + location.host + '/chat';
//         const ws = new WebSocket(uri);

//         function message(data) {
//             const line = document.createElement('p');
//             line.innerText = data;
//             chat.appendChild(line);
//         }

//         ws.onopen = function() {
//             chat.innerHTML = '<p><em>Connected!</em></p>';
//         };

//         ws.onmessage = function(msg) {
//             message(msg.data);
//         };

//         ws.onclose = function() {
//             chat.getElementsByTagName('em')[0].innerText = 'Disconnected!';
//         };

//         send.onclick = function() {
//             const msg = text.value;
//             ws.send(msg);
//             text.value = '';

//             message('<You>: ' + msg);
//         };
//         </script>
//     </body>
// </html>
// "#;


// use std::collections::HashMap;
// use std::sync::Arc;
// use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
// use tokio::net::{TcpListener, TcpStream};
// use tokio::sync::Mutex; // 替换为 Tokio 的异步 Mutex

// type SharedConnections = Arc<Mutex<HashMap<String, tokio::net::tcp::OwnedWriteHalf>>>;

// #[tokio::main]
// async fn main() -> tokio::io::Result<()> {
//     let listener = TcpListener::bind("127.0.0.1:8080").await?;
//     println!("Server is running on 127.0.0.1:8080");

//     let connections: SharedConnections = Arc::new(Mutex::new(HashMap::new()));

//     loop {
//         let (socket, _) = listener.accept().await?;
//         let connections = Arc::clone(&connections);

//         // 这里的 async block 会生成一个 `Send` 的 future
//         tokio::spawn(async move {
//             if let Err(e) = handle_client(socket, connections).await {
//                 eprintln_with_time!("Error handling client: {}", e);
//             }
//         });
//     }
// }

// async fn handle_client(
//     socket: TcpStream,
//     connections: SharedConnections,
// ) -> tokio::io::Result<()> {
//     let (reader, writer) = socket.into_split();
//     let mut reader = BufReader::new(reader);

//     let mut id = String::new();
//     reader.read_line(&mut id).await?;
//     let id = id.trim().to_string();

//     if id.is_empty() {
//         return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Empty ID"));
//     }

//     {
//         let mut conns = connections.lock().await; // 使用异步锁
//         if conns.contains_key(&id) {
//             return Err(std::io::Error::new(
//                 std::io::ErrorKind::AlreadyExists,
//                 "ID already exists",
//             ));
//         }
//         conns.insert(id.clone(), writer);
//     }

//     println!("Client '{}' connected", id);

//     let mut line = String::new();
//     while reader.read_line(&mut line).await? > 0 {
//         let message = line.trim().to_string();
//         line.clear();

//         if let Some((target_id, message)) = message.split_once(':') {
//             let target_id = target_id.trim_start_matches('@').to_string();

//             let mut conns = connections.lock().await;
//             if let Some(target_writer) = conns.get_mut(&target_id) {
//                 target_writer
//                     .write_all(format!("From {}: {}\n", id, message).as_bytes())
//                     .await?;
//             } else {
//                 let writer = conns.get_mut(&id).unwrap();
//                 writer
//                     .write_all(format!("User '{}' not found\n", target_id).as_bytes())
//                     .await?;
//             }
//         } else {
//             let mut conns = connections.lock().await;
//             let writer = conns.get_mut(&id).unwrap();
//             writer
//                 .write_all(b"Invalid message format. Use @target_id:message\n")
//                 .await?;
//         }
//     }

//     {
//         let mut conns = connections.lock().await;
//         conns.remove(&id);
//     }
//     println!("Client '{}' disconnected", id);

//     Ok(())
// }

// use futures_util::{SinkExt, StreamExt}; // 用于处理 Sink 和 Stream
// use std::collections::HashMap;
// use std::sync::{Arc, Mutex};
// use tokio_tungstenite::{accept_async, WebSocketStream};
// use tokio_tungstenite::tungstenite::protocol::Message;
// use tokio::net::TcpListener;

// type SharedConnections = Arc<Mutex<HashMap<String, tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>>>>;

// #[tokio::main]
// async fn main() -> tokio::io::Result<()> {
//     let listener = TcpListener::bind("127.0.0.1:8080").await?;
//     println!("WebSocket server is running on ws://127.0.0.1:8080");

//     let connections: SharedConnections = Arc::new(Mutex::new(HashMap::new()));

//     while let Ok((stream, _)) = listener.accept().await {
//         let connections = Arc::clone(&connections);
//         let ws_stream = accept_async(stream)
//             .await
//             .expect("Error during WebSocket handshake");

//         // 每个连接都是一个异步任务
//         tokio::spawn(handle_connection(ws_stream, connections));
//     }

//     Ok(())
// }

// async fn handle_connection(
//     ws_stream: WebSocketStream<tokio::net::TcpStream>,
//     connections: SharedConnections,
// ) {
//     // 拆分 WebSocket 流为发送 (Sink) 和接收 (Stream) 部分
//     let (mut ws_sender, mut ws_receiver) = ws_stream.split();  

//     let mut id = String::new();
    
//     // 假设客户端发送 ID
//     if let Some(Ok(Message::Text(text))) = ws_receiver.next().await {
//         id = text;  // 使用接收到的 ID
//     }

//     // 将连接存储到共享连接池
//     {
//         let mut conns = connections.lock().unwrap();
//         conns.insert(id.clone(), ws_sender.clone());
//     }

//     println!("Client '{}' connected", id);

//     // 处理接收到的消息
//     while let Some(Ok(message)) = ws_receiver.next().await {
//         if let Message::Text(text) = message {
//             if let Some((target_id, message)) = text.split_once(':') {
//                 let target_id = target_id.trim_start_matches('@').to_string();
//                 let message = message.to_string();

//                 let conns = connections.lock().unwrap();
//                 if let Some(target_sender) = conns.get(&target_id) {
//                     let response = format!("From {}: {}\n", id, message);
//                     target_sender.send(Message::Text(response)).await.unwrap();
//                 } else {
//                     let response = format!("User '{}' not found\n", target_id);
//                     ws_sender.send(Message::Text(response)).await.unwrap();
//                 }
//             } else {
//                 let response = "Invalid message format. Use @target_id:message\n".to_string();
//                 ws_sender.send(Message::Text(response)).await.unwrap();
//             }
//         }
//     }

//     // 客户端断开时从连接池中移除
//     {
//         let mut conns = connections.lock().unwrap();
//         conns.remove(&id);
//     }
//     println!("Client '{}' disconnected", id);
// }
