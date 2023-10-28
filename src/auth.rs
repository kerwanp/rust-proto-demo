use std::{
    collections::{BTreeMap, HashMap},
    env,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use bcrypt::verify;
use diesel::PgConnection;
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use protos::auth::{auth_server::Auth, LoginRequest, RegisterRequest, Token};
use sha2::Sha256;
use tonic::{Request, Response, Status};

use crate::models::{NewUser, User};

pub struct Service {
    database: Arc<Mutex<PgConnection>>,
}

impl Service {
    pub fn new(database: PgConnection) -> Self {
        Self {
            database: Arc::new(Mutex::new(database)),
        }
    }
}

#[tonic::async_trait]
impl Auth for Service {
    async fn login(&self, request: Request<LoginRequest>) -> Result<Response<Token>, Status> {
        let data = request.into_inner();

        let database = self.database.lock();

        let user = User::find_by_email(&mut database.unwrap(), &data.email)
            .ok_or(Status::unauthenticated("Invalid email or password"))?;

        match verify(data.password, &user.password) {
            Ok(true) => (),
            Ok(false) | Err(_) => return Err(Status::unauthenticated("Invalid email or password")),
        };

        let reply = generate_token(user)
            .map_err(|_| Status::unauthenticated("Invalid email or password"))?;

        Ok(Response::new(reply))
    }

    async fn register(&self, request: Request<RegisterRequest>) -> Result<Response<Token>, Status> {
        let database = self.database.lock();
        let data = request.into_inner();

        let password = bcrypt::hash(&data.password, 10)
            .map_err(|_| Status::unknown("Error while creating the user"))?;

        let user = NewUser {
            firstname: &data.firstname,
            lastname: &data.lastname,
            email: &data.email,
            password: &password,
        };

        let user = User::create(&mut database.unwrap(), user)
            .map_err(|_| Status::already_exists("User already exists in the database"))?;

        let token = generate_token(user)
            .map_err(|_| Status::unknown("Cannot generate a token for the User"))?;

        Ok(Response::new(token))
    }
}

struct GenerateClaimsError;

fn generate_claims(user: User) -> Result<BTreeMap<&'static str, String>, GenerateClaimsError> {
    let mut claims: BTreeMap<&str, String> = BTreeMap::new();

    claims.insert("sub", user.id.to_string());

    let current_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| GenerateClaimsError)?
        .as_secs();

    claims.insert("iat", current_timestamp.to_string());
    claims.insert("exp", String::from("3600"));

    Ok(claims)
}
struct GenerateTokenError;

fn generate_token(user: User) -> Result<Token, GenerateTokenError> {
    let app_key: String = env::var("APP_KEY").expect("env APP_KEY is not defined");
    let key: Hmac<Sha256> =
        Hmac::new_from_slice(app_key.as_bytes()).map_err(|_| GenerateTokenError)?;
    let claims = generate_claims(user).map_err(|_| GenerateTokenError)?;

    let access_token = claims.sign_with_key(&key).map_err(|_| GenerateTokenError)?;

    Ok(Token {
        access_token: access_token,
    })
}

pub struct VerifyTokenError;

pub fn verify_token(token: &str) -> Result<bool, VerifyTokenError> {
    let app_key: String = env::var("APP_KEY").expect("env APP_KEY is not defined");

    let key: Hmac<Sha256> =
        Hmac::new_from_slice(app_key.as_bytes()).map_err(|_| VerifyTokenError)?;

    Ok(token
        .verify_with_key(&key)
        .map(|_: HashMap<String, String>| true)
        .unwrap_or(false))
}
