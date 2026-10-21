use crate::store::Opener;

use super::locator::ProgramLocator;

/// Which operating system the presets are chosen for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetPlatform {
    /// Linux and the other Unix systems.
    Unix,
    /// Windows.
    Windows,
}

impl PresetPlatform {
    /// The platform this build runs on.
    pub const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Unix
        }
    }
}

/// How a preset is found.
enum Availability {
    /// The first of these programs that exists is the command.
    Detected(&'static [&'static str]),
    /// This command is always there.
    Always(&'static str),
}

struct Preset {
    id: &'static str,
    name: &'static str,
    availability: Availability,
    args: &'static [&'static str],
    port_ranges: &'static [(u16, u16)],
}

const VNC_PORTS: &[(u16, u16)] = &[(5900, 5999)];
const RDP_PORTS: &[(u16, u16)] = &[(3389, 3389)];

const UNIX_PRESETS: &[Preset] = &[
    Preset {
        id: "remmina-vnc",
        name: "Remmina (VNC)",
        availability: Availability::Detected(&["remmina"]),
        args: &["-c", "vnc://{host}:{port}"],
        port_ranges: VNC_PORTS,
    },
    Preset {
        id: "remmina-rdp",
        name: "Remmina (RDP)",
        availability: Availability::Detected(&["remmina"]),
        args: &["-c", "rdp://{host}:{port}"],
        port_ranges: RDP_PORTS,
    },
    Preset {
        id: "tigervnc",
        name: "TigerVNC",
        availability: Availability::Detected(&["vncviewer"]),
        args: &["{host}::{port}"],
        port_ranges: VNC_PORTS,
    },
    Preset {
        id: "freerdp",
        name: "FreeRDP",
        availability: Availability::Detected(&["xfreerdp3", "xfreerdp"]),
        args: &["/v:{host}:{port}"],
        port_ranges: RDP_PORTS,
    },
];

const WINDOWS_PRESETS: &[Preset] = &[
    Preset {
        id: "mstsc",
        name: "Remote Desktop (mstsc)",
        availability: Availability::Always("mstsc"),
        args: &["/v:{host}:{port}"],
        port_ranges: RDP_PORTS,
    },
    Preset {
        id: "tigervnc",
        name: "TigerVNC",
        availability: Availability::Detected(&[
            "vncviewer",
            r"C:\Program Files\TigerVNC\vncviewer.exe",
            r"C:\Program Files (x86)\TigerVNC\vncviewer.exe",
        ]),
        args: &["{host}::{port}"],
        port_ranges: VNC_PORTS,
    },
    Preset {
        id: "realvnc",
        name: "VNC Viewer (RealVNC)",
        availability: Availability::Detected(&[
            r"C:\Program Files\RealVNC\VNC Viewer\vncviewer.exe",
            r"C:\Program Files (x86)\RealVNC\VNC Viewer\vncviewer.exe",
        ]),
        args: &["{host}::{port}"],
        port_ranges: VNC_PORTS,
    },
];

/// The presets of `platform` whose program exists, as openers. Presets are
/// listed in their catalog order, which is the order of suggestion.
pub fn detect_presets(locator: &dyn ProgramLocator, platform: PresetPlatform) -> Vec<Opener> {
    let presets = match platform {
        PresetPlatform::Unix => UNIX_PRESETS,
        PresetPlatform::Windows => WINDOWS_PRESETS,
    };
    presets
        .iter()
        .filter_map(|preset| detect(preset, locator))
        .collect()
}

fn detect(preset: &Preset, locator: &dyn ProgramLocator) -> Option<Opener> {
    let command = match preset.availability {
        Availability::Always(command) => command,
        Availability::Detected(programs) => programs
            .iter()
            .copied()
            .find(|program| locator.is_available(program))?,
    };
    Some(Opener {
        id: preset.id.to_owned(),
        name: preset.name.to_owned(),
        command: command.to_owned(),
        args: preset.args.iter().map(|arg| (*arg).to_owned()).collect(),
        ports: preset
            .port_ranges
            .iter()
            .flat_map(|(first, last)| *first..=*last)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Installed(&'static [&'static str]);

    impl ProgramLocator for Installed {
        fn is_available(&self, program: &str) -> bool {
            self.0.contains(&program)
        }
    }

    fn ids(openers: &[Opener]) -> Vec<&str> {
        openers.iter().map(|opener| opener.id.as_str()).collect()
    }

    #[test]
    fn should_detect_nothing_on_unix_when_no_program_is_installed() {
        assert_eq!(
            detect_presets(&Installed(&[]), PresetPlatform::Unix),
            Vec::<Opener>::new()
        );
    }

    #[test]
    fn should_detect_both_remmina_presets_when_remmina_is_installed() {
        let openers = detect_presets(&Installed(&["remmina"]), PresetPlatform::Unix);

        assert_eq!(ids(&openers), ["remmina-vnc", "remmina-rdp"]);
    }

    #[test]
    fn should_use_the_newer_freerdp_program_when_both_exist() {
        let openers = detect_presets(&Installed(&["xfreerdp", "xfreerdp3"]), PresetPlatform::Unix);

        assert_eq!(openers[0].command, "xfreerdp3");
    }

    #[test]
    fn should_fall_back_to_the_older_freerdp_program() {
        let openers = detect_presets(&Installed(&["xfreerdp"]), PresetPlatform::Unix);

        assert_eq!(openers[0].command, "xfreerdp");
    }

    #[test]
    fn should_suggest_vnc_presets_for_every_display_port() {
        let openers = detect_presets(&Installed(&["vncviewer"]), PresetPlatform::Unix);

        assert_eq!(
            (
                openers[0].ports.first(),
                openers[0].ports.last(),
                openers[0].ports.len()
            ),
            (Some(&5900), Some(&5999), 100)
        );
    }

    #[test]
    fn should_always_offer_remote_desktop_on_windows() {
        let openers = detect_presets(&Installed(&[]), PresetPlatform::Windows);

        assert_eq!(ids(&openers), ["mstsc"]);
    }

    #[test]
    fn should_detect_a_vnc_viewer_by_its_install_path_on_windows() {
        let openers = detect_presets(
            &Installed(&[r"C:\Program Files\TigerVNC\vncviewer.exe"]),
            PresetPlatform::Windows,
        );

        assert_eq!(
            openers[1].command,
            r"C:\Program Files\TigerVNC\vncviewer.exe"
        );
    }

    #[test]
    fn should_not_offer_unix_presets_on_windows() {
        let openers = detect_presets(&Installed(&["remmina"]), PresetPlatform::Windows);

        assert_eq!(ids(&openers), ["mstsc"]);
    }
}
