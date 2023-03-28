mod wasm;

use zkcard::{greeter_client::GreeterClient, EchoRequest};

pub mod zkcard {
    tonic::include_proto!("zkcard");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GreeterClient::connect("http://[::1]:50051").await?;

    let request = tonic::Request::new(EchoRequest {
        name: "player".into(),
    });

    let response = client.echo(request).await?;

    println!("Response={:?}", response);

    Ok(())
}
