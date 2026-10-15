//! Re-export of the dependency-free worker<->QML protocol crate
//! [`opencode_proto`]. Kept as a thin shim so existing `crate::markers::…`
//! paths stay stable. The protocol lives in `opencode-proto/` so it can be
//! unit tested on the host without the SailfishOS toolchain (`make test`).
pub use opencode_proto::*;
