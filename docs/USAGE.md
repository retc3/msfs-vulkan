# Usage Guide

Getting started with `msfs-vulkan` is super simple. We've built both a friendly Windows GUI and a powerful command-line interface, so you can use whichever you prefer!

## What you'll need

- Windows 10 or 11.
- Rust 1.85 or newer (with the MSVC toolchain) if you're compiling from scratch.
- A decently updated GPU driver that supports Vulkan 1.3 or higher.
- A copy of Microsoft Flight Simulator (2020 or 2024) installed via Steam or the Xbox app.

*(Note: You don't need to manually download the translation DLLs anymore! The tool handles that for you.)*

## Using the GUI (Recommended)

If you just want to get in and play, this is the way to go:

1. Fire up `msfs-vulkan-gui.exe`.
2. Pick a **Configuration Preset** that matches your rig (Quality, Balanced, or Performance).
3. Hit **Apply Configuration** to generate your settings.
4. Click **Install Translation Layer**. Sit back for a few seconds while the tool automatically downloads the necessary VKD3D-Proton and DXVK files from GitHub and puts them right where they need to be.
5. Click **Run MSFS 2020 / 2024** and cross your fingers!
6. **Important:** When you're done testing, completely close the game, then click **Restore Original Files** to safely clean up the translation layer and return your game back to normal.

## Using the CLI (For Power Users)

If you like living in the terminal, we've got you covered.

### 1. Build and check your system
First, compile the tool and run a quick probe to see if your GPU is ready for Vulkan:
```powershell
cargo build --release
target\release\msfs-vulkan.exe probe
target\release\msfs-vulkan.exe discover
```

### 2. Set up your config
Let's generate your local `msfs-vulkan.toml` configuration file. If you only have one MSFS installation, it's totally automatic:
```powershell
target\release\msfs-vulkan.exe init
```

If you have multiple installs or it can't find it, just point it to your game folder directly:
```powershell
target\release\msfs-vulkan.exe init --game-dir 'C:\XboxGames\Microsoft Flight Simulator 2024\Content'
```
*(Feel free to open up `msfs-vulkan.toml` and tweak it before you install!)*

### 3. Install, Run, and Restore
*Make sure MSFS is closed before doing this!*

To install the translation layer (it will automatically download the required GitHub releases if they are missing):
```powershell
target\release\msfs-vulkan.exe install
```

To check if everything was copied correctly:
```powershell
target\release\msfs-vulkan.exe status
```

Time to fly! 
```powershell
target\release\msfs-vulkan.exe run
```
*(Pro-tip: Use `run --wait -- -SomeGameArgument` if you want to pass launch options directly to the game and wait for it to close.)*

Once you're done, safely restore your original files:
```powershell
target\release\msfs-vulkan.exe restore
```

### Where are my logs?
We keep your repository totally clean! All generated logs, shader caches, deployment states, and the crucial backups of your original game files are tucked safely away in your local AppData directory.
