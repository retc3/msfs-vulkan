# Under the Hood: Development & Architecture

Curious about how `msfs-vulkan` is put together? We built the whole project in Rust, using a modular workspace architecture to keep things clean and separated. Here's how it breaks down:

- `msfs-vulkan-gui`: The shiny Windows interface you (probably) clicked on! It uses the super lightweight `native-windows-gui` crate.
- `msfs-vulkan-cli`: The terminal equivalent for power users.
- `msfs-vulkan-core`: The heavy lifter. It handles finding your game via Steam/Xbox, downloading DLLs from GitHub, swapping files around, taking backups, and launching the sim.
- `msfs-vulkan-vulkan`: A tiny, isolated module that just pokes your Vulkan driver to see what it's capable of.

## Our Safety Guarantee

We know how scary it is to let a random tool mess with your 150GB+ flight simulator installation. We built this tool with intense paranoia to make sure we **never** break your game:

- **Strict Boundaries:** We refuse to write files anywhere outside of the configured `game-dir`. No sneaky `../` paths allowed.
- **Hash Verification:** Every single file we copy (and every backup we take) is verified with a SHA-256 hash. If it doesn't match perfectly, we abort.
- **Atomic-ish Deployments:** We write out our deployment state *before* we ever touch your game files.
- **Bulletproof Restores:** Even if your PC crashes or the deployment is interrupted midway, running a `restore --force` will use our hashes to safely reconstruct your original game state.

*Heads up: When MSFS pushes a big game update, it might overwrite or reject our translation DLLs. It's always best to hit "Restore Original Files" before downloading a sim update!*

## Contributing & Testing

If you want to fork this, build it yourself, or send us a PR, you just need standard Cargo tooling:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

We also have a sweet GitHub Actions setup in the `.github` folder that runs all these checks automatically whenever you push code. Happy hacking!
