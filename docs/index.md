---
layout: home

hero:
  name: Tunneler
  text: SSH tunnel manager
  tagline: An embedded SSH client, a tray daemon and an on-demand UI. Windows and Linux.
  image:
    src: /logo.svg
    alt: Tunneler logo
  actions:
    - theme: brand
      text: Roadmap
      link: /roadmap/
    - theme: alt
      text: GitHub
      link: https://github.com/Sonny93/tunneler

features:
  - title: One SSH session per host
    details: The SSH client is built in. Turning a tunnel on or off opens or closes a channel, it does not reconnect. It reads your ~/.ssh/config and your known_hosts.
  - title: Tags
    details: Give tunnels tags such as dev or mail, then start or stop everything under a tag in one click or one command.
  - title: Standby
    details: A tunnel in standby holds its local port and opens the SSH session only when something connects to it.
  - title: Tunnels started elsewhere
    details: Tunnels started with ssh outside Tunneler are detected. You can adopt them, kill them or import them.
  - title: Open in one click
    details: Open a tunnel in the browser, or in an app such as a VNC or RDP client picked from the target port.
  - title: Accessible
    details: The UI works with the keyboard alone and with a screen reader. The CLI works without colour and without a terminal.
---
