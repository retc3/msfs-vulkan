# msfs-vulkan

> [!CAUTION]
> **WARNING!**
> `msfs-vulkan` is highly experimental, do not expect it to increase performance, Just because your GPU supports Vulkan doesn't mean msfs-vulkan will work, MSFS may also be highly unstable while you're using msfs-vulkan. You accept the risk that your install of MSFS could be corrupted by this tool by installing it. You'll also probably see artifacting caused by the tool or crash mid way through a 14 hour flight.

> [!CAUTION]
> This is a warning, `msfs-vulkan` due to graphical glitches, MSFS may flash blue or other colors. this may trigger photosensitive epilepsy, Do not run `msfs-vulkan` if you have this condition for your own safety

Welcome!`msfs-vulkan` is a translation layer to make MSFS2020/2024's DirectX 12 calls into Vulkan,

It works by using translation layers: [KD3D-Proton](https://github.com/HansKristian-Work/vkd3d-proton) and [DXVK](https://github.com/doitsujin/dxvk) to take the MSFS DirectX calls and translate them into Vulkan calls, This may help performance on low spec systems. You can restore the original MSFS files via the CLI or the GUI.

## All the different parts

`msfs-vulkan` is broken up into 4 different parts.

- `msfs-vulkan-gui`: A friendly Graphics interface for those who like GUI's instead of Terminals.
- `msfs-vulkan-cli`: The CLI version, You'll need to call it within the Terminal
- `msfs-vulkan-core`: As the name suggests, it's the main core of `msfs-vulkan`, without it none of this would work at all.
- `msfs-vulkan-vulkan`: A tiny probe to check if your GPU can use Vulkan or not.
  
*The repos used in this project are installed by the tool, this repo does not have them.*

## Documentation

Guides incase you need em 

- [Usage Guide](docs/USAGE.md): How to use the CLI or GUI clients to use `msfs-vulkan`
- [Configuration Guide](docs/CONFIGURATION.md): How to configure `msfs-vulkan`, It's TOML settings, Performance presets, or make it point to your own fork of this project.
- [Development Guide](docs/DEVELOPMENT.md): For whoevers interested, This tells you how `msfs-vulkan` was developed, The safety boundaries the tool takes to not corrupt your install, And how to compile the code yourself if you'd like.
