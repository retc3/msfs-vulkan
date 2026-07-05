# msfs-vulkan

> [WARNING!]
>
> `msfs-vulkan` is highly experimental, do not expect it to increase performance, Just because your GPU supports vulkan doesn't mean msfs-vulkan will work, MSFS may also be highly unstable while you're using msfs-vulkan. You accept the risk that your install of MSFS could be corrupted by this tool by installing it. You'll also probably see artifacting caused by the tool or crash mid way through a 14 hour flight.

Welcome to `msfs-vulkan`! This is an experimental testing tool we built to see what happens when we run Microsoft Flight Simulator 2020 and 2024 using the Vulkan API instead of DirectX. 

How does it work? We don't actually rewrite the game's D3D12 engine. Instead, we use translation layers - specifically [VKD3D-Proton](https://github.com/HansKristian-Work/vkd3d-proton) and [DXVK](https://github.com/doitsujin/dxvk) - to intercept the game's D3D12 calls and translate them into Vulkan on the fly. This tool handles grabbing those DLLs, backing up your game files, launching the sim, and making sure everything gets restored to normal when you're done.

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
