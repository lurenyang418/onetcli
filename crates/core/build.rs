fn main() {
    for key in ["PIG_UPDATE_URL", "PIG_UPDATE_DOWNLOAD_URL"] {
        println!("cargo:rerun-if-env-changed={key}");
        if let Ok(val) = std::env::var(key) {
            if !val.is_empty() {
                println!("cargo:rustc-env={key}={val}");
            }
        }
    }
}
