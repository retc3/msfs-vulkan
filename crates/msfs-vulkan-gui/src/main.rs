#![windows_subsystem = "windows"]

use msfs_vulkan_core::{Config, Deployment, LaunchOptions, Preset, launch};
use native_windows_derive::NwgUi;
use native_windows_gui as nwg;
use native_windows_gui::NativeUi;
use std::path::PathBuf;

#[derive(Default, NwgUi)]
pub struct MsfsVulkanApp {
    #[nwg_control(size: (680, 570), position: (260, 180), title: "MSFS Vulkan", flags: "WINDOW|VISIBLE|MINIMIZE_BOX")]
    #[nwg_events( OnWindowClose: [MsfsVulkanApp::exit], OnInit: [MsfsVulkanApp::on_init] )]
    window: nwg::Window,

    installations: std::cell::RefCell<Vec<msfs_vulkan_core::discovery::GameInstallation>>,
    vkd3d_repository_values: std::cell::RefCell<Vec<String>>,
    dxvk_repository_values: std::cell::RefCell<Vec<String>>,

    #[nwg_resource(family: "Segoe UI Semibold", size: 26)]
    font_title: nwg::Font,

    #[nwg_resource(family: "Segoe UI Semibold", size: 18)]
    font_section: nwg::Font,

    #[nwg_resource(family: "Segoe UI", size: 14)]
    font_caption: nwg::Font,

    // --- Header ---
    #[nwg_control(text: "", size: (6, 66), position: (0, 12), background_color: Some([0, 120, 212]))]
    header_accent: nwg::Label,

    #[nwg_control(text: "MSFS Vulkan", size: (620, 36), position: (22, 12), font: Some(&data.font_title))]
    lbl_title: nwg::Label,

    #[nwg_control(text: "Run Microsoft Flight Simulator through a reversible D3D-to-Vulkan translation layer.", size: (630, 24), position: (24, 50), font: Some(&data.font_caption), flags: "VISIBLE|DISABLED")]
    lbl_subtitle: nwg::Label,

    // --- Configuration card ---
    #[nwg_control(size: (410, 390), position: (20, 92), flags: "VISIBLE|BORDER")]
    config_frame: nwg::Frame,

    #[nwg_control(parent: config_frame, text: "Configuration", size: (370, 26), position: (16, 10), font: Some(&data.font_section))]
    lbl_config: nwg::Label,

    #[nwg_control(parent: config_frame, text: "Choose a rendering profile and runtime sources.", size: (370, 22), position: (16, 38), font: Some(&data.font_caption), flags: "VISIBLE|DISABLED")]
    lbl_config_hint: nwg::Label,

    #[nwg_control(parent: config_frame, text: "Rendering profile", size: (180, 22), position: (16, 70))]
    lbl_preset: nwg::Label,

    #[nwg_control(parent: config_frame, text: "Quality", size: (95, 25), position: (16, 96))]
    radio_quality: nwg::RadioButton,

    #[nwg_control(parent: config_frame, text: "Balanced", check_state: nwg::RadioButtonState::Checked, size: (105, 25), position: (122, 96))]
    radio_balanced: nwg::RadioButton,

    #[nwg_control(parent: config_frame, text: "Performance", size: (125, 25), position: (244, 96))]
    radio_performance: nwg::RadioButton,

    #[nwg_control(parent: config_frame, text: "VKD3D-Proton source", size: (135, 22), position: (16, 137))]
    lbl_repo_vkd3d: nwg::Label,

    #[nwg_control(parent: config_frame, size: (235, 110), position: (153, 133))]
    combo_repo_vkd3d: nwg::ComboBox<String>,

    #[nwg_control(parent: config_frame, text: "DXVK source", size: (135, 22), position: (16, 178))]
    lbl_repo_dxvk: nwg::Label,

    #[nwg_control(parent: config_frame, size: (235, 110), position: (153, 174))]
    combo_repo_dxvk: nwg::ComboBox<String>,

    #[nwg_control(parent: config_frame, text: "Target simulator", size: (180, 22), position: (16, 221))]
    lbl_install: nwg::Label,

    #[nwg_control(parent: config_frame, size: (372, 110), position: (16, 247))]
    combo_install: nwg::ComboBox<String>,

    #[nwg_control(parent: config_frame, text: "Apply configuration", size: (372, 40), position: (16, 302))]
    #[nwg_events( OnButtonClick: [MsfsVulkanApp::apply_config] )]
    btn_apply: nwg::Button,

    #[nwg_control(parent: config_frame, text: "Selections are saved to msfs-vulkan.toml", size: (372, 22), position: (16, 352), font: Some(&data.font_caption), flags: "VISIBLE|DISABLED")]
    lbl_config_saved: nwg::Label,

    // --- Actions card ---
    #[nwg_control(size: (210, 390), position: (450, 92), flags: "VISIBLE|BORDER")]
    actions_frame: nwg::Frame,

    #[nwg_control(parent: actions_frame, text: "Actions", size: (178, 26), position: (16, 10), font: Some(&data.font_section))]
    lbl_actions: nwg::Label,

    #[nwg_control(parent: actions_frame, text: "Apply the runtime, launch the sim, or return to native DirectX.", size: (178, 52), position: (16, 38), font: Some(&data.font_caption), flags: "VISIBLE|DISABLED")]
    lbl_actions_hint: nwg::Label,

    #[nwg_control(parent: actions_frame, text: "Install translation layer", size: (178, 42), position: (16, 102))]
    #[nwg_events( OnButtonClick: [MsfsVulkanApp::install] )]
    btn_install: nwg::Button,

    #[nwg_control(parent: actions_frame, text: "Run Flight Simulator", size: (178, 42), position: (16, 154))]
    #[nwg_events( OnButtonClick: [MsfsVulkanApp::run] )]
    btn_run: nwg::Button,

    #[nwg_control(parent: actions_frame, text: "", size: (178, 1), position: (16, 216), background_color: Some([205, 205, 205]))]
    action_separator: nwg::Label,

    #[nwg_control(parent: actions_frame, text: "Restore original files", size: (178, 36), position: (16, 232))]
    #[nwg_events( OnButtonClick: [MsfsVulkanApp::restore] )]
    btn_restore: nwg::Button,

    #[nwg_control(parent: actions_frame, text: "Restore before game updates or file verification.", size: (178, 50), position: (16, 278), font: Some(&data.font_caption), flags: "VISIBLE|DISABLED")]
    lbl_restore_hint: nwg::Label,

    #[nwg_control(parent: actions_frame, text: "Status: not checked", size: (178, 36), position: (16, 336), h_align: nwg::HTextAlign::Center, background_color: Some([242, 242, 242]))]
    lbl_deployment_status: nwg::Label,

    // --- Footer status ---
    #[nwg_control(text: "Experimental mode. Close Flight Simulator before installing or restoring files.", size: (640, 46), position: (20, 500), font: Some(&data.font_caption), background_color: Some([230, 244, 255]))]
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

            config.vkd3d_repo = Self::selected_repository(
                &self.combo_repo_vkd3d,
                &self.vkd3d_repository_values,
                msfs_vulkan_core::config::DEFAULT_VKD3D_REPO,
            );
            config.dxvk_repo = Self::selected_repository(
                &self.combo_repo_dxvk,
                &self.dxvk_repository_values,
                msfs_vulkan_core::config::DEFAULT_DXVK_REPO,
            );

            if let Err(e) = config.save(&Self::config_path()) {
                nwg::modal_error_message(
                    &self.window,
                    "Error",
                    &format!("Failed to save configuration:\n{e}"),
                );
            } else {
                self.lbl_status
                    .set_text("Configuration saved. Install the translation layer when ready.");
                self.refresh_deployment_status();
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
        let saved_config = Config::load(&Self::config_path()).ok();
        let configured_vkd3d = saved_config
            .as_ref()
            .map_or(msfs_vulkan_core::config::DEFAULT_VKD3D_REPO, |config| {
                config.vkd3d_repo.as_str()
            });
        let configured_dxvk = saved_config
            .as_ref()
            .map_or(msfs_vulkan_core::config::DEFAULT_DXVK_REPO, |config| {
                config.dxvk_repo.as_str()
            });
        Self::populate_repository_combo(
            &self.combo_repo_vkd3d,
            &self.vkd3d_repository_values,
            msfs_vulkan_core::config::VKD3D_REPOSITORY_PRESETS,
            configured_vkd3d,
        );
        Self::populate_repository_combo(
            &self.combo_repo_dxvk,
            &self.dxvk_repository_values,
            msfs_vulkan_core::config::DXVK_REPOSITORY_PRESETS,
            configured_dxvk,
        );

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
        self.refresh_deployment_status();
    }

    fn populate_repository_combo(
        combo: &nwg::ComboBox<String>,
        values: &std::cell::RefCell<Vec<String>>,
        presets: &[(&str, &str)],
        configured: &str,
    ) {
        let mut repository_values = values.borrow_mut();
        let mut selected = None;

        for (label, repository) in presets {
            let index = repository_values.len();
            combo.push(format!("{label} ({repository})"));
            repository_values.push((*repository).to_owned());
            if repository.eq_ignore_ascii_case(configured) {
                selected = Some(index);
            }
        }

        if selected.is_none() && !configured.trim().is_empty() {
            selected = Some(repository_values.len());
            combo.push(format!("Custom from config ({configured})"));
            repository_values.push(configured.to_owned());
        }

        combo.set_selection(selected.or(Some(0)));
    }

    fn selected_repository(
        combo: &nwg::ComboBox<String>,
        values: &std::cell::RefCell<Vec<String>>,
        fallback: &str,
    ) -> String {
        combo
            .selection()
            .and_then(|index| values.borrow().get(index).cloned())
            .unwrap_or_else(|| fallback.to_owned())
    }

    fn refresh_deployment_status(&self) {
        let status = Config::load(&Self::config_path())
            .and_then(|config| Deployment::new(&config)?.status());
        let text = match status {
            Ok(msfs_vulkan_core::DeploymentStatus::Installed { .. }) => "Status: installed",
            Ok(msfs_vulkan_core::DeploymentStatus::NotInstalled) => "Status: not installed",
            Ok(msfs_vulkan_core::DeploymentStatus::Drifted { .. }) => "Status: needs attention",
            Err(_) => "Status: not configured",
        };
        self.lbl_deployment_status.set_text(text);
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
        self.lbl_status
            .set_text("Preparing the Vulkan runtime. This can take a moment...");
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
                            self.lbl_status.set_text(
                                "Translation layer installed. Flight Simulator is ready to launch.",
                            );
                            self.refresh_deployment_status();
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
                    if let Err(e) = deployment.restore(false) {
                        nwg::modal_error_message(
                            &self.window,
                            "Error",
                            &format!("Failed to restore:\n{e}"),
                        );
                    } else {
                        self.lbl_status
                            .set_text("Original DirectX files restored successfully.");
                        self.refresh_deployment_status();
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
                        self.lbl_status.set_text(&format!(
                            "Flight Simulator started with process ID {}.",
                            result.process_id
                        ));
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
