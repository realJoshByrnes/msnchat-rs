//! Core Tokio Sockets and Socket Manager backend.

#![allow(clippy::collapsible_if)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod manager;
pub mod socket;

pub use manager::{
    close_socket, connect_socket, create_socket, receive_socket, register_socket, send_socket,
    shutdown_socket,
};
