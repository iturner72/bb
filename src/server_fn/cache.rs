use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use crate::components::poasts::Poast;

pub static POASTS_CACHE: Lazy<Mutex<(Option<Vec<Poast>>, Instant)>> = Lazy::new(|| Mutex::new((None, Instant::now())));
pub const CACHE_DURATION: Duration = Duration::from_secs(3600);
