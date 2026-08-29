#![forbid(unsafe_code)]

fn main() {
    // The bootstrap binary deliberately performs no privileged operation.
    // Phase 3 replaces this with a system-bus D-Bus service backed by systemd's D-Bus API and Polkit.
    println!("linura-executor-systemd {} (bootstrap; no mutation backend)", env!("CARGO_PKG_VERSION"));
}
