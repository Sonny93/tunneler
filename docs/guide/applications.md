# Opening tunnels with an application

A local tunnel can be opened in your browser, or with another program: a VNC viewer, an RDP client, an SQL client. This page explains how Tunneler picks one, which programs it knows, and how to add your own.

Only local tunnels can be opened. A remote tunnel listens on the server, and a dynamic one is a proxy.

## What a tunnel opens with

When you open a tunnel, Tunneler picks the first of these that applies:

1. What you asked for: "Open with…" in a menu, a palette command, or `--with` on the command line.
2. The application saved in the tunnel's form, under "Opening" › "Application".
3. The first available application whose suggested ports contain the tunnel's **target port**.
4. The browser.

Step 3 looks at the target port, not the listening port. A tunnel that listens on `5999` and reaches port `5901` of `nova` is a VNC tunnel, and a VNC viewer is suggested for it. Your own applications come before the built-in ones. The built-in ones follow the order of the tables below.

The built-in applications suggest these ports:

| Ports | Suggested for |
|---|---|
| 5900 to 5999 | VNC |
| 3389 | RDP |

The program is started with `{host}` and `{port}` pointing at the tunnel's **listening** side. `{port}` is the local port, `5999` in the example. `{host}` is `localhost`, or `127.0.0.1` when you chose the IP address under "Opening" in the settings or in the tunnel's form. A tunnel that listens on a specific IP address uses that address.

In the tunnel's menu the open action says what will happen: "Open in the browser" or "Open with TigerVNC". "Open with…" lists the others, and the command palette has one command per application.

## Built-in applications

Tunneler looks for these programs once, when the daemon starts. After installing one, quit Tunneler and start it again. The ones it finds appear in "Settings" › "Opening" › "Applications" › "Detected on this computer".

### Windows

| Id | Application | Found | Arguments | Ports |
|---|---|---|---|---|
| `mstsc` | Remote Desktop (mstsc) | always there | `/v:{host}:{port}` | 3389 |
| `tigervnc` | TigerVNC | `vncviewer` on the `PATH`, else `C:\Program Files\TigerVNC\vncviewer.exe`, else `C:\Program Files (x86)\TigerVNC\vncviewer.exe` | `{host}::{port}` | 5900 to 5999 |
| `realvnc` | VNC Viewer (RealVNC) | `C:\Program Files\RealVNC\VNC Viewer\vncviewer.exe`, else the `Program Files (x86)` equivalent | `{host}::{port}` | 5900 to 5999 |

### Linux

| Id | Application | Found | Arguments | Ports |
|---|---|---|---|---|
| `remmina-vnc` | Remmina (VNC) | `remmina` on the `PATH` | `-c`, `vnc://{host}:{port}` | 5900 to 5999 |
| `remmina-rdp` | Remmina (RDP) | `remmina` on the `PATH` | `-c`, `rdp://{host}:{port}` | 3389 |
| `tigervnc` | TigerVNC | `vncviewer` on the `PATH` | `{host}::{port}` | 5900 to 5999 |
| `freerdp` | FreeRDP | `xfreerdp3`, else `xfreerdp`, on the `PATH` | `/v:{host}:{port}` | 3389 |

There is no built-in SQL client: the arguments depend on the client and the driver. [Add it yourself](#a-sql-client).

## Add an application

Open "Settings" (the cog in the header, or the `,` key) and go to the "Opening" tab. It holds the "Applications" section. The "Detected on this computer" list shows what Tunneler found. Under it, "Added" lists your own.

If the program you want is installed somewhere Tunneler does not look, such as a portable copy, use "Add an application":

| Field | What to enter |
|---|---|
| "Name" | The label shown in menus. |
| "Identifier" | Lowercase letters, digits and dashes. It is filled in from the name. Tunnels refer to the application by this identifier, so it cannot be changed after you add it. `browser` is reserved. |
| "Command" | The program to run, without its arguments. Give the full path when it is not on the `PATH`. |
| "Arguments" | One argument per line. |
| "Suggested ports" | Optional, separated by commas. A tunnel whose target port is in the list opens with this application by default. |

How arguments work:

- `{host}`, `{port}` and `{url}` are replaced. `{url}` is the address the browser would get, such as `http://localhost:5999/`. No other `{…}` is accepted.
- No shell runs the command. Quotes and spaces mean nothing to Tunneler: each line is passed to the program as is, and spaces in a path need no quoting.
- Variables such as `%USERPROFILE%` or `~` are not expanded.

To change one of your applications, use "Edit" next to it, then "Update". "Delete" removes it. Tunnels that were set to open with it can no longer open until you pick another application.

Nothing is saved as you go. Adding, editing and deleting only change the list in the settings window, and the settings are saved all at once with "Save" at the bottom. Closing the window by any means ("Cancel", Escape, a click outside or the close button) asks before it drops what you changed. An application form that still holds typed input blocks "Save" until you add the application or cancel the form.

### Replace a built-in application

An application of yours with the same identifier as a built-in one replaces it: the built-in one is no longer offered, and yours is used everywhere it was. Delete yours and the built-in one is back.

The quickest way is the "Customize" button next to a detected application. It opens the form filled with the built-in values and the same identifier. Change what you need and confirm.

Under the detected list, a hint says: "Installed somewhere else, like a portable version? Add it below with the full path of its program." Its "How to add an application" link opens this page.

### Example: a portable TigerVNC on Windows

You downloaded the portable TigerVNC viewer and put it in `C:\Tools\TigerVNC\vncviewer64-1.14.1.exe`. Tunneler does not find it, so a VNC tunnel opens in the browser. To make it the default viewer for VNC tunnels:

1. In "Settings" › "Opening" › "Applications", click "Add an application".
2. "Name": `TigerVNC`.
3. "Identifier": `tigervnc`. This is the identifier of the built-in TigerVNC, so yours replaces it. If you type another one, both exist side by side.
4. "Command": `C:\Tools\TigerVNC\vncviewer64-1.14.1.exe`.
5. "Arguments": one line, `{host}::{port}`. The double colon tells TigerVNC the number is a TCP port, not a display number.
6. "Suggested ports": `5900, 5901, 5902`, or the ports your VNC servers use.
7. Click "Add", then "Save" in the settings.

A tunnel from `5999` to `nova:5901` now opens with it, and the viewer gets `localhost::5999`.

The same application in `tunnels.toml`. Use single quotes around the Windows path so that the backslashes stay as they are:

```toml
[[openers]]
id = "tigervnc"
name = "TigerVNC"
command = 'C:\Tools\TigerVNC\vncviewer64-1.14.1.exe'
args = ["{host}::{port}"]
ports = [5900, 5901, 5902]
```

Tunneler reloads the file when it changes.

### A SQL client

No SQL client is built in, so a database tunnel opens in the browser unless you add one. DBeaver can connect from the command line. This opens a PostgreSQL connection to the tunnel:

```toml
[[openers]]
id = "dbeaver"
name = "DBeaver"
command = "dbeaver"
args = ["-con", "driver=postgresql|host={host}|port={port}"]
ports = [5432]
```

A tunnel whose target port is 5432 then opens with DBeaver. The `-con` syntax belongs to DBeaver: check its documentation for the driver you use.

## Choose the application for one tunnel

In the tunnel's form, the "Opening" section has an "Application" field. It offers:

- "Automatic", followed by the name of what would be picked, which is the default behaviour.
- "Browser", to always use the browser, even for a VNC port.
- Every available application.

Below the fields, a line shows the result: "Opens" followed by `http://localhost:3000/` or "Runs" followed by the command line. The same section sets the scheme (`http` or `https`), the path and the host for the browser.

In `tunnels.toml` this is `open.with`:

```toml
[[tunnels]]
host = "nova"
kind = "local"
listen_port = 5999
target_port = 5901
open = { with = "tigervnc" }
```

An identifier that matches no application is not an error when the file loads, since a program can be uninstalled. Opening that tunnel fails, and the message lists what is available.

## From the command line

```sh
tunneler open 5999                    # the tunnel's own choice
tunneler open 5999 --with tigervnc    # a given application
tunneler open 5999 --with browser     # the browser, whatever the port
```

`--with` takes `browser` or an identifier. With an identifier Tunneler does not know, `tunneler open` lists the available identifiers and exits with code 9. Exit code 8 means the tunnel did not become ready or the program could not be launched. See [exit codes](../reference/cli#exit-codes).
