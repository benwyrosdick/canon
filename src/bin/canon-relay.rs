fn main() {
    let addr = listen_addr();
    if let Err(error) = canon_3d::net::relay::run(&addr) {
        eprintln!("canon-relay failed: {error}");
        std::process::exit(1);
    }
}

fn listen_addr() -> String {
    if let Some(addr) = std::env::args().nth(1) {
        return addr;
    }
    if let Ok(addr) = std::env::var("CANON_RELAY_ADDR") {
        return addr;
    }
    let host = std::env::var("CANON_RELAY_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port = std::env::var("CANON_RELAY_PORT").unwrap_or_else(|_| "3478".into());
    format!("{host}:{port}")
}
