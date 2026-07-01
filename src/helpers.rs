//! Local id/time helpers — previously `leo_core::helpers`. Kept identical in
//! spirit so stored rows look the same.

/// RFC3339 UTC timestamp.
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// A short random hex id.
pub fn id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..12).map(|_| format!("{:x}", rng.gen_range(0..16u8))).collect()
}
