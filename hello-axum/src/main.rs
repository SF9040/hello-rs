#[cfg(not(target_env = "msvc"))]
#[cfg(feature = "jemalloc")]
use tikv_jemallocator::Jemalloc;
use tracing::error;

#[cfg(not(target_env = "msvc"))]
#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() {
    if let Err(error) = hello_axum::run().await {
        error!(error = format!("{error:#}"), "hello-axum exited with ERROR");
    };
}
