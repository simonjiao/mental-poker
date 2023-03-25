use tonic::{transport::Server, Request, Response, Status};

use zkcard::{
    greeter_server::{Greeter, GreeterServer},
    EchoReply, EchoRequest,
};

pub mod zkcard {
    tonic::include_proto!("zkcard");
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoReply>, Status> {
        println!("Got a request {:?}", request);

        let reply = zkcard::EchoReply {
            message: format!("service001 reply to {}", request.into_inner().name),
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let greeter = MyGreeter::default();

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
