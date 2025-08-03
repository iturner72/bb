use leptos::prelude::*;
use server_fn::codec::PostUrl;

#[server(
    name = InvalidatePoastsCache,
    prefix = "/api",
    endpoint = "invalidate_poasts_cache",
    input = PostUrl
)]
pub async fn invalidate_poasts_cache() -> Result<(), ServerFnError> {
    use crate::server_fn::cache::POASTS_CACHE;

    let mut cache = POASTS_CACHE.lock().unwrap();
    *cache = (None, std::time::Instant::now());

    log::info!("Poasts cache invalidated");
    Ok(())
}
