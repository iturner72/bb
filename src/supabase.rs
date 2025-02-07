use once_cell::sync::Lazy;
use postgrest::Postgrest;
use std::env;

static CLIENT: Lazy<Postgrest> = Lazy::new(|| {
    let url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");

    let client = Postgrest::new(format!("{}/rest/v1", url))
        .insert_header("apikey", &key)
        .insert_header("Authorization", format!("Bearer {}", key));

    log::debug!("Supabase client created");
    client
});

pub fn get_client() -> &'static Postgrest {
    &CLIENT
}
