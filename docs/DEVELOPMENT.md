# How `msfs-vulkan` was developed

Wondering how `msfs-vulkan` was developed? It was fully built in Rust using a modular workspace architecture, It breaks down like this.

- `msfs-vulkan-gui`: The GUI interface that most users will use which uses the `native-windows-gui` crate
- `msfs-vulkan-cli`: The terminal version of the GUI, can be helpful incase you're having issues with the GUI or like the terminal more.
- `msfs-vulkan-core`: This is what the Terminal and GUI both call, it is the base of `msfs-vulkan`, it is what finds your MSFS install, downloads the repos required, takes backups and then finally launchs MSFS2020/2024
- `msfs-vulkan-vulkan` This is what simply checks your Graphics driver to see if it supports Vulkan 1.3 or higher

## Safety Guarantee

We know that often all of you have tons of GBs of Community addons etc which is why you wouldn't trust `msfs-vulkan` to potentially corrupt all of it, which is why this tool was written to avoid corrupting it
We first refuse to write files outside of the MSFS directory, Any files copied along with backups are verified with a SHA-256 hash, if this hash doesn't match then it aborts. 
Deployment states are written before `msfs-vulkan` ever touches your game files, Even if your PC crashes or deployment is interrupted midway through you can run `restore --force` **Although, `restore --force` only works within the CLI**

>[!IMPORTANT]
>Whenever MSFS updates it may overwrite the translation DLLs, So we recommend before updating if you have the chance, Restoring your original MSFS files beforehand

## Forking, Building or Pull Requests.

If you want to fork this, compile it yourself, or send a PR our way, you'll need just standard Cargo Tooling.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

We've also setup for you a Github Actions setup within the `.github` folder that runs all checks automatically whenever you push any code.
