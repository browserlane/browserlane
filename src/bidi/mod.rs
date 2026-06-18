// Package plumbing for clicker/internal/bidi (WebDriver BiDi engine).
// Mirrors the flat Go `bidi` package by re-exporting each ported file's items.

mod browsingcontext;
mod connect;
mod connection;
mod element;
mod input;
mod protocol;
mod script;
mod session;
mod storage;

#[allow(unused_imports)]
pub use browsingcontext::*;
#[allow(unused_imports)]
pub use connect::*;
#[allow(unused_imports)]
pub use connection::*;
#[allow(unused_imports)]
pub use input::*;
#[allow(unused_imports)]
pub use protocol::*;
#[allow(unused_imports)]
pub use session::*;
#[allow(unused_imports)]
pub use storage::*;
