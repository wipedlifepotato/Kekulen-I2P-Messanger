use crate::config::config::AppConfig;
use crate::kekulenprot::Protocol;
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::services::ServeDir;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[derive(Deserialize, ToSchema)]
pub struct AddFriendRequest {
    /// Public key of the friend to add
    pub pub_key: String,
    /// Display name for the friend
    pub name: String,
}

#[derive(Deserialize, ToSchema)]
pub struct SendMessageRequest {
    /// Message text to send
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct FriendResponse {
    pub pub_key: String,
    pub name: String,
    pub messages: Vec<String>,
    pub send_count: u64,
}

#[derive(Serialize, ToSchema)]
pub struct MyInfoResponse {
    pub pub_key: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        get_friends,
        add_friend,
        get_messages,
        send_message,
        get_my_info
    ),
    components(
        schemas(AddFriendRequest, SendMessageRequest, FriendResponse, MyInfoResponse)
    ),
    tags(
        (name = "Kekulen", description = "P2P Messenger over I2P")
    )
)]
struct ApiDoc;

pub struct ApiServer {
    protocol: Arc<Protocol>,
}

impl ApiServer {
    pub fn new(protocol: Arc<Protocol>) -> Self {
        Self { protocol }
    }

    pub async fn run(&self, port: u16) {
        let api_routes = Router::new()
            .route("/friends", get(get_friends))
            .route("/friends/add", post(add_friend))
            .route("/messages/{pub_key}", get(get_messages))
            .route("/send/{pub_key}", post(send_message))
            .route("/me", get(get_my_info))
            .with_state(self.protocol.clone());

        let serve_dir = ServeDir::new("server").append_index_html_on_directories(true);

        let app = Router::new()
            .nest("/api", api_routes)
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .fallback_service(serve_dir);

        let addr = format!("127.0.0.1:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        println!("REST API started on http://{}", addr);
        println!("Swagger UI available at http://{}/swagger-ui", addr);

        axum::serve(listener, app).await.unwrap();
    }
}

#[utoipa::path(
    get,
    path = "/me",
    responses(
        (status = 200, description = "Get own public key", body = MyInfoResponse)
    )
)]
async fn get_my_info(State(protocol): State<Arc<Protocol>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "pub_key": protocol.get_my_public_key()
    }))
}

#[utoipa::path(
    get,
    path = "/friends",
    responses(
        (status = 200, description = "List all registered friends", body = [FriendResponse])
    )
)]
async fn get_friends(State(protocol): State<Arc<Protocol>>) -> Json<Vec<FriendResponse>> {
    let friends_list_arc = protocol.get_friends_list();
    let friends_guard = friends_list_arc.lock().unwrap();

    let response = friends_guard
        .iter()
        .map(|f| FriendResponse {
            pub_key: f.pub_key.clone(),
            name: f.name.clone(),
            messages: f.messages.clone(),
            send_count: f.send_count,
        })
        .collect();

    Json(response)
}

#[utoipa::path(
    post,
    path = "/friends/add",
    request_body = AddFriendRequest,
    responses(
        (status = 200, description = "Friend added and handshake initiated")
    )
)]
async fn add_friend(
    State(protocol): State<Arc<Protocol>>,
    Json(payload): Json<AddFriendRequest>,
) -> Json<serde_json::Value> {
    let friends_list_arc = protocol.get_friends_list();
    let mut friends = friends_list_arc.lock().unwrap();

    if friends.iter().any(|f| f.pub_key == payload.pub_key) {
        return Json(serde_json::json!({ "status": "error", "message": "Friend already exists" }));
    }

    let conf = AppConfig::load();

    use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
    let dummy_key = [0u8; 32];

    friends.push(crate::kekulenprot::Friend {
        pub_key: payload.pub_key,
        is_active: false,
        name: payload.name,
        key_send: ChaCha20Poly1305::new(dummy_key.as_ref().into()),
        key_recv: ChaCha20Poly1305::new(dummy_key.as_ref().into()),
        sam: Arc::new(std::sync::Mutex::new(crate::sam::sam::SAM::new(
            &conf.host_sam,
            conf.port_sam,
        ))),
        messages: Vec::new(),
        send_count: 0,
        key_exchanged: false,
        recv_count: 0,
    });

    Json(serde_json::json!({ "status": "ok", "message": "Friend added" }))
}

#[utoipa::path(
    get,
    path = "/messages/{pub_key}",
    params(
        ("pub_key" = String, Path, description = "Friend's public key")
    ),
    responses(
        (status = 200, description = "Message history for the friend")
    )
)]
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

#[utoipa::path(
    post,
    path = "/send/{pub_key}",
    params(
        ("pub_key" = String, Path, description = "Recipient's public key")
    ),
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "Message queued/sent")
    )
)]
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
