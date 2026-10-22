# Getting started

Tunneler keeps your SSH port forwards running. A small daemon holds the SSH sessions and shows a tray icon. The window is a separate program that you open when you need it and close when you don't: closing it does not stop the tunnels.

It reads your `~/.ssh/config` and your `known_hosts`, and it has its own SSH client, so you do not need `ssh` installed.

## Install

Packages are on the [releases page](https://github.com/Sonny93/tunneler/releases). They are not signed, so Windows SmartScreen warns on first run: choose "More info", then "Run anyway".

### Windows

- **Installer**, `tunneler-<version>-windows-x86_64-setup.exe`. It asks whether to install for the current user (`%LOCALAPPDATA%\Programs\Tunneler`) or for all users (`Program Files`). It installs `tunneler-ui.exe` and `tunneler.exe` side by side and adds a Start menu entry. The installer is in French.
- **Zip**, `tunneler-<version>-windows-x86_64.zip`. Extract it anywhere and run `tunneler-ui.exe`, or `tunneler.exe` for the command line. Keep the two programs in the same folder. This form is [portable](./updates-and-uninstall#portable-mode): its files stay in that folder.

### Linux

- **Debian and Ubuntu**, `tunneler_<version>_amd64.deb`, built on Ubuntu 22.04. It installs `tunneler`, `tunneler-ui` and a menu entry, and pulls in WebKitGTK 4.1 and GTK 3.

  ```sh
  sudo apt install ./tunneler_<version>_amd64.deb
  ```

- **Archive**, `tunneler-<version>-linux-x86_64.tar.gz`. Extract it and run `./tunneler-ui`, or `./tunneler` for the command line. It needs glibc 2.35 or newer, and the UI needs WebKitGTK 4.1 and GTK 3:

  ```sh
  sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0
  ```

  This form is [portable](./updates-and-uninstall#portable-mode).
- **AppImage**, `tunneler-ui-<version>-x86_64.AppImage`. Use it when you cannot install `libwebkit2gtk-4.1`. It contains the window only: put it next to `tunneler` under the name `tunneler-ui`, or set `TUNNELER_UI_BINARY` to its path.

macOS has no package and is not maintained.

## First launch

Start Tunneler from the Start menu, the application menu, or by running `tunneler-ui`. The daemon starts, a tray icon appears, and the window opens.

Close the window and the tray icon stays: the tunnels keep running. To get the window back, click the tray icon's "Open Tunneler" entry or run `tunneler` in a terminal. To stop everything, choose "Quit" in the tray menu or run `tunneler quit`.

On GNOME there is no tray by default. Install the "AppIndicator and KStatusNotifierItem Support" extension to see the icon. Without a tray, run `tunneler` to reopen the window.

## Add a first tunnel

Say `~/.ssh/config` has a host named `nova`, and a web app listens on port 3000 on that server:

```
Host nova
  HostName 203.0.113.10
  User alice
  IdentityFile ~/.ssh/id_ed25519
```

1. Click "New tunnel".
2. In "Host", type or pick `nova`. The field lists the aliases of your `ssh_config`.
3. Leave the type on "Local (-L)", and set "Listen port" to `3000`. "Target port" follows it.
4. Click "Create".

The tunnel appears in the list, grouped under `nova`. The same thing from a terminal, without saving anything:

```sh
tunneler up nova -L 3000
```

## Start it and open it

Turn the tunnel's switch on. The first time, Tunneler may ask for something:

- **A passphrase** for an encrypted key, or a password. You can let it remember the passphrase for the session or in the system keyring. Passwords are never remembered.
- **A host key** it has not seen. Compare the SHA-256 fingerprint with the one the server administrator gave you, then choose "Accept and save", "Accept once" or "Reject". A host key that changed is refused, with no way around it from the window.

When the state reads "Active", open the tunnel: use "Open in the browser" in the row's menu, or run:

```sh
tunneler open 3000
```

Your browser opens `http://localhost:3000/`. A stopped tunnel is started first. To open a tunnel with another program than the browser, such as a VNC or RDP viewer, see [Opening tunnels with an application](./applications).

## Find help

The "Documentation" button in the window's header, the `F1` key, and the "Open the documentation" command of the palette (`Ctrl+K`) open this site. In the settings, the "Applications" section links to [Opening tunnels with an application](./applications).

## Next

- [Tunnels](./tunnels): local, remote and dynamic forwards, tags and standby.
- [Command line](../reference/cli): every `tunneler` command.
