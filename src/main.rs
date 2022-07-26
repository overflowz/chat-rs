use std::{collections::HashMap, sync::Arc};

use futures::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use warp::{
    ws::{WebSocket, Ws},
    Filter, Reply,
};

#[derive(Debug, Default, serde::Serialize)]
struct Client {
    name: String,

    #[serde(skip)]
    token: String,

    #[serde(skip)]
    disconnect_timer: Option<tokio::task::JoinHandle<()>>,

    #[serde(skip)]
    tx: Option<tokio::sync::mpsc::UnboundedSender<Message>>,
}

type Clients = Arc<Mutex<HashMap<String, Client>>>;

#[derive(serde::Serialize)]
struct Message {
    from: String,
    body: String,
}

#[derive(serde::Deserialize)]
struct RegistrationRequest {
    name: String,
}

#[derive(serde::Serialize, Default)]
struct RegistrationResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'static str>,
}

#[derive(Debug, serde::Deserialize)]
struct MessageRequest {
    pub token: String,
    pub body: String,
    pub to: String,
}

async fn handle_registration(
    request: RegistrationRequest,
    clients: Clients,
) -> Result<impl warp::Reply, warp::Rejection> {
    if clients
        .lock()
        .await
        .contains_key(&request.name.to_lowercase())
    {
        return Ok(warp::reply::with_status(
            warp::reply::json(&RegistrationResponse {
                error: Some("The name is already taken"),
                ..Default::default()
            }),
            warp::http::StatusCode::NOT_ACCEPTABLE,
        ));
    }

    let token = uuid::Uuid::new_v4().as_simple().to_string();

    let client = Client {
        token: token.clone(),
        name: request.name.clone(),
        ..Default::default()
    };

    clients
        .lock()
        .await
        .insert(request.name.to_lowercase(), client);

    Ok(warp::reply::with_status(
        warp::reply::json(&RegistrationResponse {
            token: Some(token),
            ..Default::default()
        }),
        warp::http::StatusCode::OK,
    ))
}

async fn client_connected(token: String, ws: WebSocket, clients: Clients) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (client_tx, mut client_rx) = tokio::sync::mpsc::unbounded_channel();

    if let Some((_, client)) = clients
        .lock()
        .await
        .iter_mut()
        .find(|(_, client)| client.token == token)
    {
        if let Some(disconnect_timer) = client.disconnect_timer.take() {
            disconnect_timer.abort();
        }

        client.tx = Some(client_tx.clone());
    }

    tokio::spawn(async move {
        while let Some(message) = client_rx.recv().await {
            let _ = ws_tx
                .send(warp::ws::Message::text(
                    serde_json::to_string(&message).unwrap(),
                ))
                .await;
        }
    });

    // keep loop busy while socket is connected
    while ws_rx.next().await.is_some() {}

    // disconnected, clean up

    if let Some((_, client)) = clients
        .lock()
        .await
        .iter_mut()
        .find(|(_, client)| client.token == token)
    {
        let clients = clients.clone();

        client.disconnect_timer = Some(tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

            clients
                .lock()
                .await
                .retain(|_, client| client.token != token);
        }));
    }
}

async fn ws_handler(
    token: String,
    ws: Ws,
    clients: Clients,
) -> Result<impl Reply, warp::Rejection> {
    let client = clients
        .lock()
        .await
        .iter()
        .any(|(_, client)| client.token == token);

    match client {
        true => Ok(ws.on_upgrade(|socket| client_connected(token, socket, clients))),
        false => Err(warp::reject::reject()),
    }
}

async fn handle_send_message(
    request: MessageRequest,
    clients: Clients,
) -> Result<impl Reply, warp::Rejection> {
    let clients = clients.lock().await;

    let client = clients
        .get(&request.to.to_lowercase())
        .ok_or_else(warp::reject::not_found)?;

    let (_, sender_client) = clients
        .iter()
        .find(|(_, client)| client.token == request.token)
        .ok_or_else(warp::reject)?;

    let _ = client
        .tx
        .as_ref()
        .ok_or_else(warp::reject::not_found)?
        .send(Message {
            from: sender_client.name.clone(),
            body: request.body,
        });

    Ok(warp::reply())
}

async fn handle_status(
    token: String,
    clients: Clients,
) -> Result<impl warp::Reply, warp::Rejection> {
    match clients
        .lock()
        .await
        .iter()
        .find(|(_, client)| client.token == token)
    {
        Some((_, client)) => Ok(warp::reply::with_status(
            warp::reply::json(&client),
            warp::http::StatusCode::OK,
        )),
        None => Err(warp::reject()),
    }
}

async fn handle_list_clients(clients: Clients) -> Result<impl Reply, warp::Rejection> {
    let clients = clients.lock().await;
    let clients: Vec<&Client> = clients.iter().map(|(_, client)| client).collect();

    Ok(warp::reply::json(&clients))
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let clients: Clients = Arc::new(Mutex::new(HashMap::<String, Client>::new()));
    let clients = warp::any().map(move || clients.clone());

    let serve_static = warp::get().and(warp::fs::dir("static/"));

    let registration_handler = warp::path("register")
        .and(warp::post())
        .and(warp::body::json())
        .and(clients.clone())
        .and_then(handle_registration);

    let send_message_handler = warp::path("send_message")
        .and(warp::post())
        .and(warp::body::json())
        .and(clients.clone())
        .and_then(handle_send_message);

    let status_handler = warp::path("status")
        .and(warp::get())
        .and(warp::path::param())
        .and(clients.clone())
        .and_then(handle_status);

    let messages_handler = warp::path("messages")
        .and(warp::path::param())
        .and(warp::ws())
        .and(clients.clone())
        .and_then(ws_handler);

    let list_clients_handler = warp::path("clients")
        .and(warp::get())
        .and(clients.clone())
        .and_then(handle_list_clients);

    let routes = registration_handler
        .or(messages_handler)
        .or(send_message_handler)
        .or(status_handler)
        .or(list_clients_handler)
        .or(serve_static)
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_header(warp::http::header::CONTENT_TYPE)
                .allow_methods(vec![warp::http::Method::GET, warp::http::Method::POST]),
        );

    println!("starting http server");
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}
