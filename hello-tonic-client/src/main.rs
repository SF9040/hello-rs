#[tokio::main]
async fn main() {
    if let Err(e) = hello_tonic_client::run().await {
        eprintln!("hello-tonic-client exited with ERROR: {e:#}");
    };
}
