# Updates and uninstall

## Updates

When the window opens, Tunneler asks GitHub once whether a newer release exists. If the check fails, for example offline, nothing is shown. Nothing else in Tunneler contacts the network on its own, and nothing is installed without you clicking.

When a newer version exists, a banner shows its release notes, a "View release" link and a "Hide" button that hides it until the window is reopened.

| Form | What the banner offers |
|---|---|
| Windows installer | "Install and restart". It asks for confirmation when tunnels are running: they stop, and they start again afterwards if "Restore active tunnels on launch" is on. It downloads the installer with a progress text, checks its signature, stops the daemon, runs the installer and relaunches Tunneler. |
| Windows zip, Linux archive, Debian package, AppImage | The link only. Download the new version and replace the old one yourself. For the package: `sudo apt install ./tunneler_<version>_amd64.deb`. |

The AppImage never updates itself because it does not contain the daemon.

To check at any moment, even with the automatic check off, click "Check now" under "Updates" in the General tab of the settings, or run "Check for updates" from the command palette (`Ctrl+K`). A line next to the button says whether you have the latest version, which version is available, or that the check failed. A version found this way shows the banner again, even if you hid it.

Turn the automatic check off with "Check for updates when opening" in the settings, or in `tunnels.toml`:

```toml
[settings]
check_for_updates = false
```

Updates are signed. An installed Tunneler accepts only updates signed with the key it shipped with.

A per-machine Windows install asks for administrator rights at each update. The installer stops a running daemon before it copies files.

## Where the files go

Tunneler writes `tunnels.toml`, `state.toml`, logs and the window's web data. [Configuration](../reference/configuration#file-locations) lists the paths for each system. Passphrases you asked it to remember go to the system keyring, and "Launch Tunneler at login" adds an entry that the system keeps.

## Portable mode

The Windows zip and the Linux archive contain an empty file named `portable` next to the programs. While it is there, Tunneler keeps its files in that folder instead of your user directories:

| File | Portable | Installed on Linux |
|---|---|---|
| `tunnels.toml` | next to the programs | `~/.config/tunneler/` |
| `state.toml` | `data/` | `~/.local/state/tunneler/` |
| Logs | `data/logs/` | `~/.local/share/tunneler/logs/` |
| Window web data | `data/webview/` | `~/.local/share/com.minimalstuff.tunneler/` |

Delete the `portable` file to go back to the user directories. The files already written are not moved. `--config` and `--state`, or `TUNNELER_CONFIG` and `TUNNELER_STATE`, win over both.

The launch-at-login entry and the passphrases in the system keyring stay outside the folder. `tunneler purge` removes them.

Keep `tunneler` and `tunneler-ui` in the same folder: the daemon looks for `tunneler-ui` next to itself, then on the `PATH`, then at the path in `TUNNELER_UI_BINARY`.

## Uninstall

### `tunneler purge`

Removing the programs leaves your files behind. `tunneler purge` removes what Tunneler left on the machine, except the programs. It stops the daemon, then removes:

- the launch-at-login entry;
- the logs and the window's web data;
- `tunnels.toml` and `state.toml`;
- the passphrases in the system keyring that `state.toml` lists;
- the `tunneler` directories left empty.

It never touches the programs, the `portable` file, the install directory or `~/.ssh`, and it never removes a file you gave with `--config` or `--state`.

```sh
tunneler purge --dry-run      # list what would be removed, remove nothing
tunneler purge                # list, ask for confirmation, remove
tunneler purge --yes          # no question; required without a terminal, otherwise exit code 5
tunneler purge --keep-config  # keep tunnels.toml, state.toml and the passphrases
```

Use `--keep-config` when you plan to reinstall and want your tunnels back.

### Windows installer

Open "Settings", "Apps", "Tunneler", then "Uninstall". The installer is in French. Its confirmation page has a checkbox, « Supprimer aussi mes tunnels, réglages et passphrases mémorisées », meaning "Also delete my tunnels, settings and remembered passphrases".

- Ticked: it runs `tunneler purge --yes`, and everything goes.
- Unticked: it runs `tunneler purge --yes --keep-config`. Your tunnels, settings and passphrases stay for a later reinstall, and the rest is cleaned.

An update never purges. A per-machine install is uninstalled by an administrator, and its uninstaller only cleans the data of the user who runs it, since the data belongs to each user.

### Debian and Ubuntu

Run `tunneler purge` first, then remove the package. `apt` never touches your home directory, so the package cannot clean your files itself.

```sh
tunneler purge
sudo apt remove tunneler
```

### Portable

Run `tunneler purge`, then delete the folder.

