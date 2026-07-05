# Configuring `msfs-vulkan.toml`

`msfs-vulkan` uses a configuration file called `msfs-vulkan.toml`. This file is what tells the tool how to set up the translation layer and what environment variables it has to pass to MSFS

## Defaults
When you click **Apply Configuration**  in the GUI or run the command `init` in the CLI it generates a default manifest telling the tool where to put the translation Dlls next to MSFS's exe file.

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
*(MSFS still creates some DirectX11 objects even though its DirectX12 usually, that's why we have DXVK's `d3d11.dll`)*

## Advanced Targeting

MSFS ships with an Agility SDK core within `D3D12/D3D12Core.dll`. Although <ins>**We strongly recommend leaving this alone until you've tested the default setup and checked it's logs**</ins>

Although, If you figure out that the Agility core is where `msfs-vulkan` really needs to translate you can manually target it.

```toml
[[files]]
source = "d3d12core.dll"
target = "D3D12/D3D12Core.dll"
```
*We've made a backup of your original files already in case anything goes wrong*

## Repository Source Dropdowns
The GUI exposes VKD3D-Proton and DXVK as non editable dropdowns.
**Apply Configuration** saves the repository values into `msfs-vulkan.toml`


The built-in list contains repositories maintained by the project.
`runtime.lock.json` is a binary provenance file and does not control GUI choices.


## Using your own forks.

If you're a developer and would like to use your own custom fork of VKD3D-Proton or DXVK, You can simply tell the tool to download your fork instead of the official repos.

You do not put your custom fork within the GUI, Open `msfs-vulkan.toml` and
edit these lines with your own forks link:
```toml
vkd3d-repo = "HansKristian-Work/vkd3d-proton"
dxvk-repo = "doitsujin/dxvk"
```

After you restart the GUI, an unrecognized value such as your fork will show up as **Custom from config** 
in the dropdown. Applying the Configuration saves that custom value, The next installation you do will install the latest release archive from the configured GitHub Repo.

## Presets.

VKD3D and DXVK have tons of environment variables, To make it simple to install. we've put in three presets that will populate the `[environment]` block of the TOML file automatically.

### Quality
Everything is enabled, Raytracing translation (`dxr11`), DirectX 12 feature level `12_2`, state caching and full debug logging.

### Balanced (**Recommended**)
Leaves Raytracing translation (`dxr11`), DirectX12 feature level `12_2` but turns off any heavy debug logging.

### Performance 
If you're struggling on the above 2 presets, use this preset. It disables Raytracing translation (`nodxr`) along with Resizable BAR uploads (`no_upload_hvv`), DirectX 12 feature level drops from `12_2` to `12_1` and logging stays disabled.
