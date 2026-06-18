use interprocess::local_socket::tokio::Listener;
use interprocess::local_socket::{GenericFilePath, ListenerOptions, ToFsName};

/// Creates a local-socket listener (a Windows named pipe, e.g. `\\.\pipe\browserlane`)
/// at `socket_path`.
pub fn listen(socket_path: &str) -> std::io::Result<Listener> {
    let name = socket_path.to_fs_name::<GenericFilePath>()?;
    ListenerOptions::new().name(name).create_tokio()
}
