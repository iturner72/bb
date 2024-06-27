use postgrest::Postgrest;
use once_cell::sync::Lazy;
use std::env;

static CLIENT: Lazy<Postgrest> = Lazy::new(|| {
    let url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");
    Postgrest::new(url).insert_header("apikey",key)
});

pub fn get_client() -> &'static Postgrest {
    &CLIENT
}
