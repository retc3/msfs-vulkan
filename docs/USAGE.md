# Usage Guide

Using `msfs-vulkan` is pretty easy, You can use the GUI or a CLI, Whichever you like!

## What you'll need

- Windows 11/10 (Minimum Windows 10 v1909)
- Rust 1.85 and up with the MSVC toolchain if you're looking to compile this.
- A GPU that supports Vulkan 1.3 or higher. You may simply have to update your Drivers to meet this requirement
- Microsoft Flight Simulator 2020/2024 from Steam or the Microsoft Store/Xbox App


## GUI (Recommended for basic users)

If you just wanna try it out via the GUI like most people

1. Run `msfs-vulkan-gui.exe` from the latest release
2. Select a **Configuration Preset** for your rig, e.g if you have a mid range from 6 years ago, select Performance, High end from 5 years ago Balanced and so on.
3. Click **Apply Configuration**
4. Click **Install Translation Layers** as this project doesn't come with them. Wait while your PC installs them from their Github repos. `msfs-vulkan` will not work without these installed. If installed the status in the GUI will change to Installed and then you may move on.
5. Click **Run Flight Simulator** and wait for the game to launch. if it throws a error, Make a issue in the Github.
6. **Important:** When you're finished testing, Click **Restore Original Files** to remove the translation layers and restore your sim to DirectX 12/11.

## CLI (incase you're having issues with the GUI or would like the Terminal instead)

If you'd like to use the terminal instead or are having issues with the GUI,

### 1. Build and verify your system meets the Vulkan 1.3 requirement
Compile the tool and verify your system meets the Vulkan 1.3 requirement via the commands listed.
```powershell
cargo build --release
target\release\msfs-vulkan.exe probe
target\release\msfs-vulkan.exe discover
```

### 2. Set your Configuration.
To generate your `msfs-vulkan.toml` configuration file, You must run this command. If you only have one Installation of MSFS this is automatic
```powershell
target\release\msfs-vulkan.exe init
```

Although if you have multiple installs or `msfs-vulkan-core` cannot find it you can use the `--game-dir` command to point it to your install.
```powershell
target\release\msfs-vulkan.exe init --game-dir 'C:\XboxGames\Microsoft Flight Simulator 2024\Content'
```
*(As long as you know what you're doing, You may feel free to open `msfs-vulkan.toml` and tweak it before installation)*

### 3. Installing, Running, and Restoring original files.
*Make sure MSFS is closed before starting, Check Task Manager incase `FlightSimulator.exe` is open in the background.*

To start installation of the translation layer, If you're missing the releases required it'll automatically install them for you.
```powershell
target\release\msfs-vulkan.exe install
```

To make sure it was installed correctly you'll need to run this before starting MSFS
```powershell
target\release\msfs-vulkan.exe status
```

Running  MSFS with `msfs-vulkan` installed
```powershell
target\release\msfs-vulkan.exe run
```
*(Use `run --wait -- -yourgamearguement if you want to pass arguments to MSFS directly and then wait for it to close.)*

If you'd like to uninstall `msfs-vulkan` from MSFS use this command.
```powershell
target\release\msfs-vulkan.exe restore
```

### Where are the logs?
All logs, shader caches, deployment state and backups are stored under the app's local AppData profile directory.
