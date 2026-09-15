# G13 Nexus

A complete Rust-based solution for using the Logitech G13 Advanced Gameboard on Linux. Features a background daemon for event handling and a graphical configuration center powered by `eframe`/`egui`.

## Features
- **Hardware Support:** Reads input from the G13 thumbstick and all 22 G-keys.
- **RGB Control:** Adjust the backlight color and brightness directly from the GUI.
- **Macro Recording:** On-the-fly macro recording using the `MR` key or through the GUI.
- **Profile Management:** Create, save, and switch between multiple JSON configuration profiles.

## Installation

**RPM-based distributions (Nobara, Fedora, RHEL):**
Download the latest `.rpm` from the Releases page and install:

    sudo dnf install ./g13-nexus-*.rpm

## Usage

1. Start the daemon (requires root privileges to create uinput virtual devices):

       sudo g13-daemon

2. Open the Configuration GUI (can be run as your standard user):

       g13-gui
