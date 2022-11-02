#[tokio::main]
async fn main() {
    if let Err(error) = hello_tonic_client::run().await {
        eprintln!("hello-tonic-client exited with ERROR: {error:#}");
    };
}
