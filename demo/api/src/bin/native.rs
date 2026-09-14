use rundtisch::adapters::platform::native::NativePlatform;
use rundtisch::runtime::native::serve;
use rundtisch::AppState;
use rundtisch_demo::build_router;

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let platform = NativePlatform::new();
    let state = AppState::from_platform(&platform);
    serve(build_router(state)).await;
}
