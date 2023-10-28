use std::env;

use diesel::{Connection, PgConnection};
use dotenvy::dotenv;
use protos::{auth::auth_server::AuthServer, greeting::greeting_server::GreetingServer};
use tonic::transport::Server;

mod auth;
mod greeting;
mod models;
mod schema;

pub fn connect_db() -> PgConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let database = connect_db();
    let addr = "[::1]:50051".parse()?;

    Server::builder()
        .add_service(AuthServer::new(auth::Service::new(database)))
        .add_service(GreetingServer::new(greeting::Service::default()))
        .serve(addr)
        .await?;

    Ok(())
}
