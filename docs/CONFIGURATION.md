# Tweaking Your Configuration

Behind the scenes, `msfs-vulkan` uses a configuration file called `msfs-vulkan.toml`. This file is the holy grail for telling the tool exactly how to set up the translation layer and what environment variables to pass to the game.

## The Default Setup

When you click "Apply Configuration" in the GUI or run `init` in the CLI, we generate a default manifest for you. It tells the tool to place the four vital translation DLLs right next to your game's executable:

```toml
[[files]]
source = "d3d12.dll"
target = "d3d12.dll"

[[files]]
source = "d3d12core.dll"
target = "d3d12core.dll"

[[files]]
source = "d3d11.dll"
target = "d3d11.dll"

[[files]]
source = "dxgi.dll"
target = "dxgi.dll"
```
*(Fun fact: MSFS still creates some D3D11 objects when it starts up, even though it's primarily a D3D12 game. That's why we need DXVK's `d3d11.dll` in there!)*

## Advanced Targeting (For the brave)

MSFS actually ships an Agility SDK core tucked away at `D3D12/D3D12Core.dll`. **We strongly recommend leaving this alone at first.** Test out the default setup and check your logs. 

However, if you're experimenting and find out that the Agility core is where the translation really needs to happen, you can manually target it like this:

```toml
[[files]]
source = "d3d12core.dll"
target = "D3D12/D3D12Core.dll"
```
*Don't worry, we always securely back up your original files before making any changes!*

## Repository Source Dropdowns

The GUI exposes VKD3D-Proton and DXVK sources as non-editable dropdowns. Clicking
**Apply Configuration** saves the selected repository values into
`msfs-vulkan.toml`.

The built-in list intentionally contains repositories maintained by the project.
`runtime.lock.json` remains a binary provenance file and does not control GUI
choices.

## Pointing to Your Own Custom Forks

This is where the magic happens! If you're a developer and you've created your own custom fork of VKD3D-Proton or DXVK specifically tailored for MSFS, you don't need to manually copy your files around. 

Custom repositories are not entered through the GUI. Open `msfs-vulkan.toml` and
edit these lines instead:
```toml
vkd3d-repo = "HansKristian-Work/vkd3d-proton"
dxvk-repo = "doitsujin/dxvk"
```

After restarting the GUI, an unrecognized value appears as **Custom from config**
in the relevant dropdown. Applying the configuration preserves and saves that
custom value. The next install downloads the latest compatible release archive
from the configured GitHub repository.

## Environment Presets

VKD3D-Proton and DXVK have dozens of environment variables. To make life easy, we've baked in three presets that automatically populate the `[environment]` block of your TOML file:

### 🌟 Quality
Turns on everything! You get Raytracing translation (`dxr11`), D3D12 feature level `12_2`, state caching, and full debug logging so you can see exactly what's happening under the hood.

### ⚖️ Balanced (The Default)
Keeps the good stuff like Raytracing (`dxr11`) and D3D12 feature level `12_2`, but turns off all the heavy logging so your SSD isn't getting hammered.

### 🚀 Performance (For Low-End Rigs)
If you're struggling for frames, use this. It completely disables the expensive Raytracing translation path (`nodxr`), turns off Resizable BAR uploads (`no_upload_hvv`), drops the feature level to `12_1`, and keeps logging disabled. It's the best way to squeeze out every drop of performance!
