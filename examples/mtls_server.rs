use a2a_rust_sdk::models::{A2aResponse, AgentAuthentication, MessageRole};
use a2a_rust_sdk::server::{axum_router, TaskManager};
use axum_server::tls_rustls::RustlsConfig;
use rustls::server::WebPkiClientVerifier;
use rustls::RootCertStore;
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let mut manager = TaskManager::new(None);

    manager.set_on_message_received(Arc::new(|params| {
        let mut reply = params.message;
        reply.role = MessageRole::Agent;
        Ok(A2aResponse::Message(reply))
    }));

    let mut card = a2a_rust_sdk::models::AgentCard::new("mTLS Agent", "https://127.0.0.1:5443");
    card.authentication = Some(AgentAuthentication {
        schemes: vec!["mTLS".to_string(), "Bearer".to_string()],
        credentials: Some("demo-token".to_string()),
    });
    manager.set_agent_card(card);

    let app = axum_router(Arc::new(manager));

    let cert = "examples/certs/server.pem";
    let key = "examples/certs/server.key";
    let ca = "examples/certs/ca.pem";

    let cert_chain = {
        let mut reader = BufReader::new(File::open(cert).expect("open server cert"));
        certs(&mut reader)
            .collect::<Result<Vec<_>, _>>()
            .expect("read server cert chain")
    };
    let key = {
        let mut reader = BufReader::new(File::open(key).expect("open server key"));
        private_key(&mut reader)
            .expect("read server key")
            .expect("missing server private key")
    };
    let mut roots = RootCertStore::empty();
    {
        let mut reader = BufReader::new(File::open(ca).expect("open client CA"));
        for cert in certs(&mut reader) {
            roots.add(cert.expect("read client CA")).expect("add client CA");
        }
    }
    let client_verifier = WebPkiClientVerifier::builder(Arc::new(roots))
        .build()
        .expect("build client verifier");
    let server_config = rustls::ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(cert_chain, key)
        .expect("bad certificate/key");
    let config = RustlsConfig::from_config(Arc::new(server_config));

    println!("mTLS server listening on https://127.0.0.1:5443");
    axum_server::bind_rustls("127.0.0.1:5443".parse().unwrap(), config)
        .serve(app.into_make_service())
        .await
        .expect("serve mTLS");
}
