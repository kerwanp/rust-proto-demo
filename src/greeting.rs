use protos::greeting::{greeting_server::Greeting, GreetRequest, GreetResponse};
use tonic::{Request, Response, Status};

use crate::auth;

#[derive(Default)]
pub struct Service {}

#[tonic::async_trait]
impl Greeting for Service {
    async fn greet(
        &self,
        request: Request<GreetRequest>,
    ) -> Result<Response<GreetResponse>, Status> {
        let token = request
            .metadata()
            .get("x-authorization")
            .ok_or(Status::unauthenticated("No access token specified"))?
            .to_str()
            .map_err(|_| Status::unauthenticated("No access token specified"))?;

        match auth::verify_token(token) {
            Ok(true) => (),
            Err(_) | Ok(false) => return Err(Status::unauthenticated("Invalid token")),
        }

        let data = request.into_inner();

        Ok(Response::new(GreetResponse {
            message: format!("{} {}", data.message, "Pong!"),
        }))
    }
}
