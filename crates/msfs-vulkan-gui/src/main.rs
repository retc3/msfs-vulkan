#![windows_subsystem = "windows"]

use msfs_vulkan_core::{Config, Deployment, LaunchOptions, Preset, launch};
use native_windows_derive::NwgUi;
use native_windows_gui as nwg;
use native_windows_gui::NativeUi;
use std::path::PathBuf;

#[derive(Default, NwgUi)]
pub struct MsfsVulkanApp {
    #[nwg_control(size: (450, 400), position: (300, 300), title: "MSFS Vulkan Translation Layer", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [MsfsVulkanApp::exit], OnInit: [MsfsVulkanApp::on_init] )]
    window: nwg::Window,

    installations: std::cell::RefCell<Vec<msfs_vulkan_core::discovery::GameInstallation>>,

    // --- Configuration Section ---
    #[nwg_control(text: "Configuration", size: (200, 20), position: (10, 10))]
    lbl_config: nwg::Label,

    #[nwg_control(text: "Quality Preset", size: (180, 25), position: (20, 30))]
    radio_quality: nwg::RadioButton,

    #[nwg_control(text: "Balanced Preset (Default)", check_state: nwg::RadioButtonState::Checked, size: (180, 25), position: (20, 60))]
    radio_balanced: nwg::RadioButton,

    #[nwg_control(text: "Performance Preset", size: (180, 25), position: (20, 90))]
    radio_performance: nwg::RadioButton,

    #[nwg_control(text: "Apply Configuration", size: (160, 30), position: (20, 120))]
    #[nwg_events( OnButtonClick: [MsfsVulkanApp::apply_config] )]
    btn_apply: nwg::Button,

    // --- Repository Section ---
    #[nwg_control(text: "VKD3D-Proton Repo:", size: (130, 20), position: (10, 160))]
    lbl_repo_vkd3d: nwg::Label,

    #[nwg_control(text: msfs_vulkan_core::config::DEFAULT_VKD3D_REPO, size: (270, 25), position: (150, 160))]
    txt_repo_vkd3d: nwg::TextInput,

    #[nwg_control(text: "DXVK Repo:", size: (130, 20), position: (10, 195))]
    lbl_repo_dxvk: nwg::Label,

    #[nwg_control(text: msfs_vulkan_core::config::DEFAULT_DXVK_REPO, size: (270, 25), position: (150, 195))]
    txt_repo_dxvk: nwg::TextInput,

    // --- Installation Section ---
    #[nwg_control(text: "Target Simulator:", size: (130, 20), position: (10, 230))]
    lbl_install: nwg::Label,

    #[nwg_control(size: (270, 25), position: (150, 230))]
    combo_install: nwg::ComboBox<String>,

    // --- Actions Section ---
    #[nwg_control(text: "Actions", size: (210, 20), position: (220, 10))]
    lbl_actions: nwg::Label,

    #[nwg_control(text: "Install Translation Layer", size: (190, 30), position: (230, 30))]
    #[nwg_events( OnButtonClick: [MsfsVulkanApp::install] )]
    btn_install: nwg::Button,

    #[nwg_control(text: "Restore Original Files", size: (190, 30), position: (230, 70))]
    #[nwg_events( OnButtonClick: [MsfsVulkanApp::restore] )]
    btn_restore: nwg::Button,

    #[nwg_control(text: "Run MSFS 2020 / 2024", size: (190, 30), position: (230, 120))]
    #[nwg_events( OnButtonClick: [MsfsVulkanApp::run] )]
    btn_run: nwg::Button,

    // --- Bottom Status Label ---
    #[nwg_control(text: "Allows testing MSFS 2020/2024 through a D3D12-to-Vulkan translation layer.\nPlease close MSFS before installing or restoring!", size: (430, 80), position: (10, 300), flags: "VISIBLE")]
    lbl_status: nwg::Label,
}

impl MsfsVulkanApp {
    #[allow(clippy::unused_self)]
    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }

    fn config_path() -> PathBuf {
        PathBuf::from("msfs-vulkan.toml")
    }

    fn apply_config(&self) {
        let preset = if self.radio_quality.check_state() == nwg::RadioButtonState::Checked {
            Preset::Quality
        } else if self.radio_performance.check_state() == nwg::RadioButtonState::Checked {
            Preset::Performance
        } else {
            Preset::Balanced
        };

        let installations = self.installations.borrow();
        if installations.is_empty() {
            nwg::modal_error_message(
                &self.window,
                "Error",
                "No MSFS 2020 or 2024 installation found. Cannot apply configuration.",
            );
            return;
        }

        let selected_idx = self.combo_install.selection().unwrap_or(0);
        if let Some(installation) = installations.get(selected_idx) {
            let mut config =
                Config::new(installation.game_dir.clone(), PathBuf::from("runtime/x64"));
            config.environment = preset.environment();

            let vkd3d = self.txt_repo_vkd3d.text();
            if !vkd3d.trim().is_empty() {
                config.vkd3d_repo = vkd3d;
            }

            let dxvk = self.txt_repo_dxvk.text();
            if !dxvk.trim().is_empty() {
                config.dxvk_repo = dxvk;
            }

            if let Err(e) = config.save(&Self::config_path()) {
                nwg::modal_error_message(
                    &self.window,
                    "Error",
                    &format!("Failed to save configuration:\n{e}"),
                );
            } else {
                nwg::modal_info_message(
                    &self.window,
                    "Success",
                    "Configuration applied successfully.",
                );
            }
        } else {
            nwg::modal_error_message(&self.window, "Error", "Selected installation is invalid.");
        }
    }

    fn on_init(&self) {
        match msfs_vulkan_core::discover_installations() {
            Ok(found) => {
                let labels: Vec<String> = found
                    .iter()
                    .map(|inst| {
                        let version = if inst
                            .executable
                            .file_name()
                            .is_some_and(|n| n == "FlightSimulator2024.exe")
                        {
                            "MSFS 2024"
                        } else {
                            "MSFS 2020"
                        };
                        format!("{} ({}) - {}", version, inst.store, inst.game_dir.display())
                    })
                    .collect();

                for label in &labels {
                    self.combo_install.push(label.clone());
                }
                if !labels.is_empty() {
                    self.combo_install.set_selection(Some(0));
                }

                *self.installations.borrow_mut() = found;
            }
            Err(e) => {
                nwg::modal_error_message(
                    &self.window,
                    "Error",
                    &format!("Failed to discover MSFS installations:\n{e}"),
                );
            }
        }
    }

    fn install(&self) {
        let path = Self::config_path();
        if !path.exists() {
            nwg::modal_error_message(
                &self.window,
                "Error",
                "Configuration not found. Please click 'Apply Configuration' first.",
            );
            return;
        }
        match Config::load(&path) {
            Ok(config) => {
                if let Err(e) = msfs_vulkan_core::download::ensure_runtime(&config) {
                    nwg::modal_error_message(
                        &self.window,
                        "Error",
                        &format!("Failed to download runtime:\n{e}"),
                    );
                    return;
                }
                match Deployment::new(&config) {
                    Ok(deployment) => {
                        if let Err(e) = deployment.install() {
                            nwg::modal_error_message(
                                &self.window,
                                "Error",
                                &format!("Failed to install:\n{e}"),
                            );
                        } else {
                            nwg::modal_info_message(
                                &self.window,
                                "Success",
                                "Translation layer installed successfully.",
                            );
                        }
                    }
                    Err(e) => {
                        nwg::modal_error_message(
                            &self.window,
                            "Error",
                            &format!("Deployment error:\n{e}"),
                        );
                    }
                }
            }
            Err(e) => {
                nwg::modal_error_message(
                    &self.window,
                    "Error",
                    &format!("Failed to load configuration:\n{e}"),
                );
            }
        }
    }

    fn restore(&self) {
        let path = Self::config_path();
        if !path.exists() {
            nwg::modal_error_message(&self.window, "Error", "Configuration not found.");
            return;
        }
        match Config::load(&path) {
            Ok(config) => match Deployment::new(&config) {
                Ok(deployment) => {
                    if let Err(e) = deployment.restore(true) {
                        nwg::modal_error_message(
                            &self.window,
                            "Error",
                            &format!("Failed to restore:\n{e}"),
                        );
                    } else {
                        nwg::modal_info_message(
                            &self.window,
                            "Success",
                            "Original files restored successfully.",
                        );
                    }
                }
                Err(e) => {
                    nwg::modal_error_message(
                        &self.window,
                        "Error",
                        &format!("Deployment error:\n{e}"),
                    );
                }
            },
            Err(e) => {
                nwg::modal_error_message(
                    &self.window,
                    "Error",
                    &format!("Failed to load configuration:\n{e}"),
                );
            }
        }
    }

    fn run(&self) {
        let path = Self::config_path();
        if !path.exists() {
            nwg::modal_error_message(
                &self.window,
                "Error",
                "Configuration not found. Please configure and install first.",
            );
            return;
        }
        match Config::load(&path) {
            Ok(config) => {
                let options = LaunchOptions {
                    arguments: vec![],
                    wait: false,
                    allow_uninstalled: false,
                };
                match launch(&config, &options) {
                    Ok(result) => {
                        nwg::modal_info_message(
                            &self.window,
                            "Success",
                            &format!("Started MSFS (PID: {})", result.process_id),
                        );
                    }
                    Err(e) => {
                        nwg::modal_error_message(
                            &self.window,
                            "Error",
                            &format!("Failed to launch:\n{e}"),
                        );
                    }
                }
            }
            Err(e) => {
                nwg::modal_error_message(
                    &self.window,
                    "Error",
                    &format!("Failed to load configuration:\n{e}"),
                );
            }
        }
    }
}

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");

    let mut font = nwg::Font::default();
    nwg::Font::builder()
        .family("Segoe UI")
        .size(16)
        .build(&mut font)
        .expect("Failed to build font");

    nwg::Font::set_global_default(Some(font));

    let _app = MsfsVulkanApp::build_ui(MsfsVulkanApp::default()).expect("Failed to build UI");
    nwg::dispatch_thread_events();
}
