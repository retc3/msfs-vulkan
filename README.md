# msfs-vulkan

> [!CAUTION]
> **WARNING!**
> `msfs-vulkan` is highly experimental, do not expect it to increase performance, Just because your GPU supports vulkan doesn't mean msfs-vulkan will work, MSFS may also be highly unstable while you're using msfs-vulkan. You accept the risk that your install of MSFS could be corrupted by this tool by installing it. You'll also probably see artifacting caused by the tool or crash mid way through a 14 hour flight.

Welcome!`msfs-vulkan` is a translation layer to make MSFS2020/2024's DirectX 12 calls into Vulkan,

It works by using translation layers [KD3D-Proton](https://github.com/HansKristian-Work/vkd3d-protonand) and [DXVK](https://github.com/doitsujin/dxvkto) to take the MSFS DirectX calls and translate them into Vulkan calls, This may help performance on Linux systems and some low spec systems. You can restore the original MSFS files via the CLI or the GUI.

## What's in the box?

We broke the project down into a few different pieces:

- `msfs-vulkan-gui`: A friendly, easy-to-use Windows interface to manage everything.
- `msfs-vulkan-cli`: If you prefer the terminal, this is the command-line version!
- `msfs-vulkan-core`: The brains of the operation. It finds your Steam/Xbox installation, manages your configuration, handles downloading the translation layers, and keeps your original files safe.
- `msfs-vulkan-vulkan`: A tiny, read-only probe that just checks what your Vulkan driver is capable of.

*Note: We don't actually ship the third-party translation DLLs in this repo. The tool downloads them automatically for you when you're ready to play.*

## Documentation

Want to dive deeper? Check out our guides:

- [Usage Guide](docs/USAGE.md): How to actually use the GUI and CLI to run the game.
- [Configuration Guide](docs/CONFIGURATION.md): How to tweak the TOML settings, use performance presets, or even point to your own custom GitHub forks!
- [Development Guide](docs/DEVELOPMENT.md): For the nerds - how we built this, our safety guarantees, and how to compile the code yourself.
