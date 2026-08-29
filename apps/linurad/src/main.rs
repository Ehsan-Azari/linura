#![forbid(unsafe_code)]

use linura_control::ControlPlane;
use linura_policy::BaselinePolicy;

fn main() {
    let _control = ControlPlane::new(BaselinePolicy);
    println!(
        "linurad {} local control-plane bootstrap initialized",
        env!("CARGO_PKG_VERSION")
    );
}
