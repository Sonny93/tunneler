//! Windows: the icon needs a message loop on its thread, which a `winit`
//! event loop provides. Commands wake it up as user events.
//!
//! This code is only cross-compiled in development; it has never been run.

use std::io;
use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::platform::windows::EventLoopBuilderExtWindows;
use winit::window::WindowId;

use super::TRAY_THREAD_NAME;
use crate::daemon::tray::command::{RunningTray, TrayCommand, TrayCommandSender, TrayThreadGone};
use crate::daemon::tray::native::{NativeTray, forward_events};
use crate::daemon::tray::{TrayAction, TrayStatus, TrayStatusCell, TrayView};

/// How long to wait for the tray thread to create its event loop.
const EVENT_LOOP_TIMEOUT: Duration = Duration::from_secs(5);

type BuiltSender = oneshot::Sender<Result<(), String>>;

pub(in crate::daemon::tray) fn spawn(
    initial_view: TrayView,
    actions: mpsc::UnboundedSender<TrayAction>,
    status: Arc<TrayStatusCell>,
    built: BuiltSender,
) -> io::Result<RunningTray> {
    let (proxy_sender, proxy_receiver) = channel();
    let thread = std::thread::Builder::new()
        .name(TRAY_THREAD_NAME.to_owned())
        .spawn(move || run(initial_view, actions, status, built, &proxy_sender))?;
    let proxy = proxy_receiver
        .recv_timeout(EVENT_LOOP_TIMEOUT)
        .map_err(io::Error::other)?
        .map_err(io::Error::other)?;
    Ok(RunningTray {
        commands: TrayCommandSender::new(move |command| {
            proxy.send_event(command).map_err(|_| TrayThreadGone)
        }),
        thread: Some(thread),
    })
}

fn run(
    initial_view: TrayView,
    actions: mpsc::UnboundedSender<TrayAction>,
    status: Arc<TrayStatusCell>,
    built: BuiltSender,
    proxy_sender: &Sender<Result<EventLoopProxy<TrayCommand>, String>>,
) {
    forward_events(actions);
    let mut builder = EventLoop::<TrayCommand>::with_user_event();
    builder.with_any_thread(true);
    let event_loop = match builder.build() {
        Ok(event_loop) => event_loop,
        Err(error) => {
            let _ = proxy_sender.send(Err(error.to_string()));
            let _ = built.send(Err(error.to_string()));
            return;
        }
    };
    event_loop.set_control_flow(ControlFlow::Wait);
    let _ = proxy_sender.send(Ok(event_loop.create_proxy()));
    let mut application = TrayApplication {
        initial_view,
        status,
        built: Some(built),
        tray: None,
    };
    if let Err(error) = event_loop.run_app(&mut application) {
        tracing::warn!(%error, "the tray event loop ended abnormally");
    }
}

struct TrayApplication {
    initial_view: TrayView,
    status: Arc<TrayStatusCell>,
    built: Option<BuiltSender>,
    tray: Option<NativeTray>,
}

impl TrayApplication {
    fn report(&mut self, outcome: Result<(), String>) {
        if let Some(built) = self.built.take() {
            let _ = built.send(outcome);
        }
    }
}

impl ApplicationHandler<TrayCommand> for TrayApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.tray.is_some() {
            return;
        }
        match NativeTray::build(&self.initial_view) {
            Ok(tray) => {
                self.tray = Some(tray);
                self.status.set(TrayStatus::Active);
                self.report(Ok(()));
            }
            Err(message) => {
                self.report(Err(message));
                event_loop.exit();
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, command: TrayCommand) {
        match command {
            TrayCommand::Render(view) => {
                if let Some(tray) = &self.tray {
                    tray.show(&view);
                }
            }
            TrayCommand::Shutdown => event_loop.exit(),
        }
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}
