//! Every Win32 call in the product, and the composition that turns them into
//! collector factories.
//!
//! Nothing outside this directory may name a Windows API. That is what keeps
//! collection *policy* testable on any host against a scripted provider, and
//! what keeps the privilege boundary small enough to audit by reading it.

pub mod composition;
pub mod devices;
pub mod eventlog;
pub mod managed_file_lock;
pub mod memory;
pub mod named_pipe;
pub mod network;
pub mod process;
pub mod registry;
pub mod resource_probe;
pub mod resume;
pub mod safe_file;
pub mod services;
pub mod socket_rows;
pub mod sockets;
pub mod tasks;
pub mod tool_probe;
pub mod wua;
