#![forbid(unsafe_code)]

fn main() {
    if let Err(error) = linura_executor_systemd::serve() {
        eprintln!("linura-executor-systemd failed closed: {error}");
        std::process::exit(1);
    }
}
