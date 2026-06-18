use std::time::Duration;

use anyhow::anyhow;
use interprocess::local_socket::tokio::{prelude::*, Stream};
use interprocess::local_socket::{GenericFilePath, ToFsName};

/// Connects to the daemon socket.
pub async fn dial(addr: &str, timeout: Duration) -> anyhow::Result<Stream> {
    let name = addr.to_fs_name::<GenericFilePath>()?;
    match tokio::time::timeout(timeout, Stream::connect(name)).await {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(anyhow!("dial timeout after {timeout:?}")),
    }
}
