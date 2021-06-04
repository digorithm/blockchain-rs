use actix_web::middleware::Logger;
use actix_web::{web, App, Error, HttpResponse, HttpServer, Responder};
use log::info;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{Arc, Mutex};

use blockchain::blockchain::Blockchain;

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionRequest {
    pub amount: f64,
    pub sender: String,
    pub recipient: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterNodeRequest {
    pub addresses: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct AppState {
    blockchain: Arc<Mutex<Blockchain>>,
}

// AppState holds the state that want to share across the many different endpoints.
// In this case it's just the blockchain that lives in-memory.
impl AppState {
    fn new() -> Result<Self, Error> {
        let blockchain = Blockchain::new();
        Ok(AppState {
            blockchain: Arc::new(Mutex::new(blockchain)),
        })
    }
}

async fn register(
    state: web::Data<AppState>,
    req: web::Json<RegisterNodeRequest>,
) -> impl Responder {
    let mut blockchain = state.blockchain.lock().unwrap();

    for node in &req.addresses {
        blockchain.register_node(node.to_owned());
    }

    HttpResponse::Ok().body(format!(
        "Total nodes registered: {}\n",
        blockchain.nodes.len()
    ))
}

async fn resolve(state: web::Data<AppState>) -> impl Responder {
    let mut blockchain = state.blockchain.lock().unwrap();
    let replaced = blockchain.resolve_conflicts();

    if replaced {
        let response = format!("Chain replaced. New chain:\n{:?}", blockchain.chain);
        HttpResponse::Ok().body(response)
    } else {
        HttpResponse::Ok().body("Chain not replaced\n")
    }
}

async fn mine(state: web::Data<AppState>) -> impl Responder {
    let mut blockchain = state.blockchain.lock().unwrap();
    let new_block = blockchain.mine();
    HttpResponse::Ok().json(new_block)
}

async fn transaction(
    state: web::Data<AppState>,
    tx: web::Json<TransactionRequest>,
) -> Result<HttpResponse, Error> {
    let mut b = state.blockchain.lock().unwrap();

    let next_block = b.new_transaction(tx.sender.clone(), tx.recipient.clone(), tx.amount);
    info!("Current txs: {:?}", b.current_transactions);

    Ok(HttpResponse::Ok().body(format!("Next block: {}", next_block)))
}

async fn chain(state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let blockchain = state.blockchain.lock().unwrap();
    let chain = blockchain.chain.clone();
    Ok(HttpResponse::Ok().json(chain))
}

/// This is bitcoin-like node that exposes a HTTP interface.
/// The blockchain-related logic lives in `lib.rs`, so
/// it would be fairly easy to expose it to different interfaces, such as RPC.

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Creates the blockchain
    env::set_var("RUST_LOG", "info,actix_todo=debug,actix_web=info");
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let port: u16 = args[1].parse().unwrap();

    let app_state = AppState::new().unwrap();

    HttpServer::new(move || {
        App::new()
            .data(app_state.clone())
            .wrap(Logger::default())
            .service(web::resource("/nodes/register").route(web::post().to(register)))
            .service(web::resource("/nodes/resolve").route(web::get().to(resolve)))
            .service(web::resource("/mine").route(web::get().to(mine)))
            .service(web::resource("/chain").route(web::get().to(chain)))
            .service(web::resource("/transactions/new").route(web::post().to(transaction)))
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
