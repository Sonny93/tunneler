//! X11 forwarding: finding the local display, reading its cookie from the
//! `Xauthority` file, and serving the X11 channels the server opens.

mod command;
mod cookie;
mod display;
mod environment;
mod error;
mod local;
mod local_connection;
mod router;
mod serve;
mod setup;
mod xauthority;

pub(crate) use command::{
    ANCHOR_COMMAND, ChannelEvent, next_channel_event, read_display_line, request_x11_forwarding,
    start_command,
};
pub use environment::{DisplayPlatform, SystemX11Environment, X11Environment};
pub use error::X11Error;
pub(crate) use local::LocalX11Resolver;
pub(crate) use router::{LocalX11, X11Forwarding, X11Router};
pub(crate) use serve::{X11Connection, serve_x11_connection};
