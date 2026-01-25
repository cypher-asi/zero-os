//! VFS IPC clients

pub mod async_ops;
mod blocking;

pub use blocking::{VfsClient, VFS_ENDPOINT_SLOT, VFS_RESPONSE_SLOT};
