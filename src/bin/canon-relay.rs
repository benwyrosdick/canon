fn main() {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:3478".into());
    if let Err(error) = canon_3d::net::relay::run(&addr) {
        eprintln!("canon-relay failed: {error}");
        std::process::exit(1);
    }
}
