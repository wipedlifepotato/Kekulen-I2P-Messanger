use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::kekulenprot::Protocol;
use crate::config::config::AppConfig; // Исправлено: добавлена ;

#[derive(Deserialize)]
pub struct AddFriendRequest {
    pub pub_key: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub message: String,
}

#[derive(Serialize)]
pub struct FriendResponse {
    pub pub_key: String,
    pub name: String,
    pub messages: Vec<String>,
    pub send_count: u64,
}

pub struct ApiServer {
    protocol: Arc<Protocol>,
}

impl ApiServer {
    pub fn new(protocol: Arc<Protocol>) -> Self {
        Self { protocol }
    }

    pub async fn run(self, port: u16) {
        let app = Router::new()
            .route("/friends", get(get_friends))
            .route("/friends/add", post(add_friend))
            .route("/messages/{pub_key}", get(get_messages))
            .route("/send/{pub_key}", post(send_message))
            .route("/me", get(get_my_info))
            .with_state(self.protocol);

        let addr = format!("127.0.0.1:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        println!("REST API started on http://{}", addr);
        axum::serve(listener, app).await.unwrap();
    }
}

// --- Обработчики (Handlers) ---
async fn get_my_info(
    State(protocol): State<Arc<Protocol>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "pub_key": protocol.get_my_public_key()
    }))
}
/// Получить список всех друзей
async fn get_friends(
    State(protocol): State<Arc<Protocol>>,
) -> Json<Vec<FriendResponse>> {
    let friends_list_arc = protocol.get_friends_list();
    let friends_guard = friends_list_arc.lock().unwrap();
    
    let response = friends_guard.iter().map(|f| FriendResponse {
        pub_key: f.pub_key.clone(),
        name: f.name.clone(),
        messages: f.messages.clone(),
        send_count: f.send_count,
    }).collect();

    Json(response)
}

/// Добавить нового друга в список для дозвона
async fn add_friend(
    State(protocol): State<Arc<Protocol>>,
    Json(payload): Json<AddFriendRequest>,
) -> Json<serde_json::Value> {
    let friends_list_arc = protocol.get_friends_list();
    let mut friends = friends_list_arc.lock().unwrap();
    
    if friends.iter().any(|f| f.pub_key == payload.pub_key) {
        return Json(serde_json::json!({ "status": "error", "message": "Friend already exists" }));
    }

    // Загружаем настройки SAM из конфига
    let conf = AppConfig::load();
    
    use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
    let dummy_key = [0u8; 32];
    
    friends.push(crate::kekulenprot::Friend {
        pub_key: payload.pub_key,
        is_active: false,
        name: payload.name,
        key_send: ChaCha20Poly1305::new(dummy_key.as_ref().into()),
        key_recv: ChaCha20Poly1305::new(dummy_key.as_ref().into()),
        sam: Arc::new(std::sync::Mutex::new(crate::sam::sam::SAM::new(&conf.host_sam, conf.port_sam))),
        messages: Vec::new(),
        send_count: 0,
        key_exchanged: false,
        recv_count: 0,
    });

    Json(serde_json::json!({ "status": "ok", "message": "Friend added" }))
}

/// Получить историю сообщений конкретного друга
async fn get_messages(
    State(protocol): State<Arc<Protocol>>,
    Path(pub_key): Path<String>,
) -> Json<serde_json::Value> {
    let friends_list_arc = protocol.get_friends_list();
    let friends = friends_list_arc.lock().unwrap();
    
    if let Some(f) = friends.iter().find(|f| f.pub_key == pub_key) {
        Json(serde_json::json!({
            "name": f.name,
            "messages": f.messages
        }))
    } else {
        Json(serde_json::json!({ "status": "error", "message": "Friend not found" }))
    }
}

/// Отправить сообщение через I2P
async fn send_message(
    State(protocol): State<Arc<Protocol>>,
    Path(pub_key): Path<String>,
    Json(payload): Json<SendMessageRequest>,
) -> Json<serde_json::Value> {
    match protocol.send_to(&pub_key, &payload.message) {
        Ok(_) => Json(serde_json::json!({ "status": "ok" })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e })),
    }
}