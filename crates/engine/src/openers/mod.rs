//! The programs that open a tunnel besides the browser: the presets detected
//! on this machine, the openers the user added, the default choice for a
//! tunnel and the command line that is run.

mod launch;
mod locator;
mod preset;
mod resolution;

pub use launch::{LaunchCommand, OpenPlan};
pub use locator::{ProgramLocator, SystemProgramLocator};
pub use preset::{PresetPlatform, detect_presets};
pub use resolution::{
    AvailableOpener, DefaultOpener, OpenerSource, available_openers, default_opener,
};

pub(crate) use launch::plan_open;
pub(crate) use resolution::{OpenAction, choose_action};
