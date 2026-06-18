// Package plumbing for clicker/internal/api.
// One module per ported file; populated in later phases.

mod actionability;
mod clock_script;
mod handlers_a11y;
mod handlers_capture;
mod handlers_clock;
mod handlers_dialog;
mod handlers_download;
mod handlers_elements;
mod handlers_emulation;
mod handlers_frames;
mod handlers_input;
mod handlers_interaction;
mod handlers_lifecycle;
mod handlers_navigation;
mod handlers_network;
mod handlers_recording;
mod handlers_state;
mod handlers_storage;
mod handlers_websocket;
mod helpers;
mod pipe;
mod recording;
mod router;
mod server;
mod session;
mod version;

#[allow(unused_imports)]
pub use actionability::*;
#[allow(unused_imports)]
pub use clock_script::*;
#[allow(unused_imports)]
pub use handlers_a11y::*;
#[allow(unused_imports)]
pub use handlers_capture::*;
#[allow(unused_imports)]
pub use handlers_clock::*;
#[allow(unused_imports)]
pub use handlers_dialog::*;
#[allow(unused_imports)]
pub use handlers_frames::*;
#[allow(unused_imports)]
pub use handlers_elements::*;
#[allow(unused_imports)]
pub use handlers_interaction::*;
#[allow(unused_imports)]
pub use handlers_emulation::*;
#[allow(unused_imports)]
pub use handlers_lifecycle::*;
#[allow(unused_imports)]
pub use handlers_navigation::*;
#[allow(unused_imports)]
pub use handlers_recording::*;
#[allow(unused_imports)]
pub use recording::*;
#[allow(unused_imports)]
pub use handlers_state::*;
#[allow(unused_imports)]
pub use handlers_storage::*;
#[allow(unused_imports)]
pub use helpers::*;
#[allow(unused_imports)]
pub use pipe::*;
#[allow(unused_imports)]
pub use router::*;
#[allow(unused_imports)]
pub use server::*;
#[allow(unused_imports)]
pub use session::*;
