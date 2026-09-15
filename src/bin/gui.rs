#[path = "../config.rs"]
mod config;

use config::{G13Config, get_profiles_dir};
use eframe::egui;
use std::{fs, path::PathBuf, sync::{Arc, Mutex}, thread, time::Duration};

fn get_active_path() -> PathBuf {
    dirs::home_dir().unwrap().join(".config/g13_nexus/active_profile.json")
}

struct G13App {
    config: G13Config,
    profiles: Vec<String>,
    selected_profile: String,
    new_profile_name: String,
    active_key: String,
    edit_value: String,
    active_m_slot: String,
    is_recording: bool,
    is_macro_recording: bool,
    macro_target_key: String,
    recorded_macro_steps: Vec<String>,
    #[allow(dead_code)]
    macros: Arc<Mutex<std::collections::HashMap<String, Vec<String>>>>,
    last_pressed_key: Arc<Mutex<String>>,
}

impl Default for G13App {
    fn default() -> Self {
        let profiles_dir = get_profiles_dir();
        let _ = fs::create_dir_all(&profiles_dir);
        let profiles: Vec<String> = fs::read_dir(&profiles_dir)
            .map(|entries| {
                entries.filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
                    .map(|e| e.file_name().to_string_lossy().replace(".json", ""))
                    .collect()
            })
            .unwrap_or_default();

        let config: G13Config = fs::read_to_string(get_active_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let selected_profile = config.profile_name.clone();
        let last_pressed_key = Arc::new(Mutex::new(String::new()));
        let macros = Arc::new(Mutex::new(std::collections::HashMap::new()));

        let lp_clone = Arc::clone(&last_pressed_key);
        thread::spawn(move || {
            let api = hidapi::HidApi::new().ok();
            if let Some(api) = api {
                loop {
                    if let Some(info) = api.device_list().find(|d| d.vendor_id() == 0x046d && d.product_id() == 0xc21c) {
                        if let Ok(device) = info.open_device(&api) {
                            let _ = device.set_blocking_mode(true);
                            let mut buf = [0u8; 64];
                            let mut prev = [0u8; 8];
                            loop {
                                if let Ok(res) = device.read(&mut buf) {
                                    if res >= 8 {
                                        let keys_map = [
                                            ("TOP1", 6, 0x02), ("TOP2", 6, 0x04), ("TOP3", 6, 0x08), ("TOP4", 6, 0x10),
                                            ("M1", 6, 0x20), ("M2", 6, 0x40), ("M3", 6, 0x80), ("MR", 7, 0x01),
                                            ("EXTRA1", 7, 0x02), ("EXTRA2", 7, 0x04), ("STICK_CLICK", 7, 0x08),
                                            ("G1", 3, 0x01), ("G2", 3, 0x02), ("G3", 3, 0x04), ("G4", 3, 0x08),
                                            ("G5", 3, 0x10), ("G6", 3, 0x20), ("G7", 3, 0x40), ("G8", 3, 0x80),
                                            ("G9", 4, 0x01), ("G10", 4, 0x02), ("G11", 4, 0x04), ("G12", 4, 0x08),
                                            ("G13", 4, 0x10), ("G14", 4, 0x20), ("G15", 4, 0x40), ("G16", 4, 0x80),
                                            ("G17", 5, 0x01), ("G18", 5, 0x02), ("G19", 5, 0x04), ("G20", 5, 0x08),
                                            ("G21", 5, 0x10), ("G22", 5, 0x20),
                                        ];

                                        for &(k, idx, mask) in &keys_map {
                                            let was = (prev[idx] & mask) != 0;
                                            let is_pressed = (buf[idx] & mask) != 0;
                                            if is_pressed && !was {
                                                if let Ok(mut lock) = lp_clone.lock() { *lock = k.to_string(); }
                                            }
                                        }

                                        let stick_x = buf[1];
                                        let stick_y = buf[2];
                                        
                                        let up_pressed = stick_y < 50;
                                        let down_pressed = stick_y > 200;
                                        let left_pressed = stick_x < 50;
                                        let right_pressed = stick_x > 200;

                                        let prev_stick_x = prev[1];
                                        let prev_stick_y = prev[2];
                                        let prev_up = prev_stick_y < 50;
                                        let prev_down = prev_stick_y > 200;
                                        let prev_left = prev_stick_x < 50;
                                        let prev_right = prev_stick_x > 200;

                                        if up_pressed && !prev_up {
                                            if let Ok(mut lock) = lp_clone.lock() { *lock = "STICK_UP".to_string(); }
                                        }
                                        if down_pressed && !prev_down {
                                            if let Ok(mut lock) = lp_clone.lock() { *lock = "STICK_DOWN".to_string(); }
                                        }
                                        if left_pressed && !prev_left {
                                            if let Ok(mut lock) = lp_clone.lock() { *lock = "STICK_LEFT".to_string(); }
                                        }
                                        if right_pressed && !prev_right {
                                            if let Ok(mut lock) = lp_clone.lock() { *lock = "STICK_RIGHT".to_string(); }
                                        }

                                        prev.copy_from_slice(&buf[..8]);
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                    thread::sleep(Duration::from_secs(1));
                }
            }
        });

        Self {
            config,
            profiles,
            selected_profile,
            new_profile_name: String::new(),
            active_key: String::new(),
            edit_value: String::new(),
            active_m_slot: "M1".to_string(),
            is_recording: false,
            is_macro_recording: false,
            macro_target_key: String::new(),
            recorded_macro_steps: Vec::new(),
            macros,
            last_pressed_key,
        }
    }
}

impl G13App {
    fn save_current(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.config) {
            let _ = fs::create_dir_all(get_active_path().parent().unwrap());
            let _ = fs::write(get_active_path(), &json);
            let profile_path = get_profiles_dir().join(format!("{}.json", self.config.profile_name));
            let _ = fs::write(profile_path, &json);
        }
    }

    fn load_profile(&mut self, name: &str) {
        let path = get_profiles_dir().join(format!("{}.json", name));
        if let Ok(data) = fs::read_to_string(path) {
            if let Ok(cfg) = serde_json::from_str(&data) {
                self.config = cfg;
                self.selected_profile = name.to_string();
                self.save_current();
            }
        }
    }

    fn delete_profile(&mut self, name: &str) {
        if self.profiles.len() <= 1 { return; }
        let profile_path = get_profiles_dir().join(format!("{}.json", name));
        let _ = fs::remove_file(profile_path);
        
        self.profiles.retain(|p| p != name);
        if let Some(new_sel) = self.profiles.first().cloned() {
            self.load_profile(&new_sel);
        }
    }

    fn render_key_button(&self, ui: &mut egui::Ui, key: &str, width: f32, height: f32) -> egui::Response {
        let binding = self.config.keys.get(key).cloned().unwrap_or_default();
        let display_binding = if binding.starts_with("MACRO:") { "Macro".to_string() } else { binding };

        let mut button = egui::Button::new("");
        if self.active_key == key || (self.is_macro_recording && self.macro_target_key == key) {
            button = button.fill(egui::Color32::from_rgb(60, 120, 180));
        }

        let resp = ui.add_sized([width, height], button);
        let rect = resp.rect;
        let painter = ui.painter();
        let font_id = egui::FontId::proportional(12.0);
        let color = if self.active_key == key { egui::Color32::WHITE } else { ui.style().visuals.text_color() };

        if display_binding.is_empty() {
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, key, font_id, color);
        } else {
            painter.text(rect.center() + egui::vec2(0.0, -8.0), egui::Align2::CENTER_CENTER, key, font_id.clone(), color);
            painter.text(rect.center() + egui::vec2(0.0, 8.0), egui::Align2::CENTER_CENTER, display_binding, font_id, egui::Color32::LIGHT_GRAY);
        }

        resp
    }
}

impl eframe::App for G13App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Ok(mut lock) = self.last_pressed_key.lock() {
            if !lock.is_empty() {
                let pressed = lock.clone();
                lock.clear();
                
                if self.is_macro_recording {
                    if pressed != self.macro_target_key {
                        self.recorded_macro_steps.push(pressed);
                    }
                } else {
                    if pressed == "MR" {
                        self.is_recording = !self.is_recording;
                    } else if pressed.starts_with('M') && pressed.len() == 2 {
                        self.active_m_slot = pressed.clone();
                    } else if self.is_recording {
                        if let Ok(mut m_lock) = self.macros.lock() {
                            let slot_key = format!("{}_{}", self.active_m_slot, pressed);
                            let entry = m_lock.entry(slot_key).or_default();
                            entry.push(self.config.keys.get(&pressed).cloned().unwrap_or(pressed.clone()));
                        }
                    } else {
                        self.active_key = pressed.clone();
                        self.edit_value = self.config.keys.get(&pressed).cloned().unwrap_or_default();
                    }
                }
            }
        }

        ui.ctx().request_repaint_after(Duration::from_millis(100));

        ui.heading("G13 Nexus Command Center");

        ui.horizontal(|ui| {
            ui.label("Profile:");
            let current_sel = self.selected_profile.clone();
            let mut profile_to_load = None;

            egui::ComboBox::from_label("")
                .selected_text(&current_sel)
                .show_ui(ui, |ui| {
                    for p in &self.profiles {
                        let is_selected = self.selected_profile == *p;
                        if ui.selectable_value(&mut self.selected_profile, p.clone(), p).clicked() && !is_selected {
                            profile_to_load = Some(p.clone());
                        }
                    }
                });

            if let Some(pname) = profile_to_load {
                self.load_profile(&pname);
            }

            if ui.button("Delete Profile").clicked() && self.profiles.len() > 1 {
                let p_to_del = self.selected_profile.clone();
                self.delete_profile(&p_to_del);
            }

            ui.text_edit_singleline(&mut self.new_profile_name);
            if ui.button("Create Profile").clicked() && !self.new_profile_name.is_empty() {
                let name = self.new_profile_name.clone();
                self.config.profile_name = name.clone();
                if !self.profiles.contains(&name) { self.profiles.push(name); }
                self.save_current();
                self.new_profile_name.clear();
            }
        });

        ui.horizontal(|ui| {
            let mut rgb = self.config.color;
            ui.color_edit_button_srgb(&mut rgb);
            if rgb != self.config.color { self.config.color = rgb; }

            ui.label("Brightness:");
            let mut max_val = *rgb.iter().max().unwrap_or(&255).max(&1) as f32;
            if ui.add(egui::Slider::new(&mut max_val, 0.0..=255.0)).changed() {
                let old_max = *rgb.iter().max().unwrap_or(&1) as f32;
                if old_max > 0.0 {
                    let scale = max_val / old_max;
                    rgb[0] = (rgb[0] as f32 * scale).clamp(0.0, 255.0) as u8;
                    rgb[1] = (rgb[1] as f32 * scale).clamp(0.0, 255.0) as u8;
                    rgb[2] = (rgb[2] as f32 * scale).clamp(0.0, 255.0) as u8;
                } else {
                    rgb = [max_val as u8, max_val as u8, max_val as u8];
                }
                self.config.color = rgb;
            }

            if ui.button("Apply Color & Save").clicked() {
                self.save_current();
            }
        });

        ui.separator();

        let btn_w = 68.0;
        let btn_h = 48.0;
        let total_row_w = 7.0 * btn_w + 6.0 * 8.0;

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Top Display Keys");
                ui.horizontal(|ui| {
                    let top_w = 4.0 * btn_w + 3.0 * 8.0;
                    let pad = (total_row_w - top_w) / 2.0;
                    if pad > 0.0 { ui.add_space(pad); }
                    for &k in &["TOP1", "TOP2", "TOP3", "TOP4"] {
                        if self.render_key_button(ui, k, btn_w, 32.0).clicked() {
                            self.active_key = k.to_string();
                            self.edit_value = self.config.keys.get(k).cloned().unwrap_or_default();
                        }
                    }
                });

                ui.add_space(5.0);
                ui.label("M-Keys & MR");
                ui.horizontal(|ui| {
                    let m_w = 4.0 * btn_w + 3.0 * 8.0;
                    let pad = (total_row_w - m_w) / 2.0;
                    if pad > 0.0 { ui.add_space(pad); }
                    for &k in &["M1", "M2", "M3", "MR"] {
                        let binding = self.config.keys.get(k).cloned().unwrap_or_default();
                        let display_binding = if binding.starts_with("MACRO:") { "Macro".to_string() } else { binding };

                        let mut button = egui::Button::new("");
                        if k == "MR" && self.is_recording {
                            button = button.fill(egui::Color32::from_rgb(180, 50, 50));
                        } else if self.active_key == k {
                            button = button.fill(egui::Color32::from_rgb(60, 120, 180));
                        }

                        let resp = ui.add_sized([btn_w, 38.0], button);
                        let rect = resp.rect;
                        let painter = ui.painter();
                        let font_id = egui::FontId::proportional(12.0);
                        let color = if self.active_key == k || (k == "MR" && self.is_recording) { egui::Color32::WHITE } else { ui.style().visuals.text_color() };

                        let display_label = if k.starts_with('M') && self.active_m_slot == k { format!("{}\n[ACTIVE]", k) } else if k == "MR" && self.is_recording { "MR\n[REC]".to_string() } else if display_binding.is_empty() { k.to_string() } else { format!("{}\n{}", k, display_binding) };

                        if display_label.contains('\n') {
                            let mut parts = display_label.split('\n');
                            let p1 = parts.next().unwrap_or(k);
                            let p2 = parts.next().unwrap_or("");
                            painter.text(rect.center() + egui::vec2(0.0, -8.0), egui::Align2::CENTER_CENTER, p1, font_id.clone(), color);
                            painter.text(rect.center() + egui::vec2(0.0, 8.0), egui::Align2::CENTER_CENTER, p2, font_id, egui::Color32::LIGHT_GRAY);
                        } else {
                            painter.text(rect.center(), egui::Align2::CENTER_CENTER, display_label, font_id, color);
                        }

                        if resp.clicked() {
                            if k == "MR" { self.is_recording = !self.is_recording; } else if k.starts_with('M') { self.active_m_slot = k.to_string(); }
                            self.active_key = k.to_string();
                            self.edit_value = self.config.keys.get(k).cloned().unwrap_or_default();
                        }
                    }
                });

                ui.add_space(10.0);
                ui.label("G-Key Matrix");

                let rows = [
                    &["G1", "G2", "G3", "G4", "G5", "G6", "G7"][..],
                    &["G8", "G9", "G10", "G11", "G12", "G13", "G14"],
                    &["G15", "G16", "G17", "G18", "G19"],
                    &["G20", "G21", "G22"],
                ];

                for row in rows {
                    ui.horizontal(|ui| {
                        let row_w = row.len() as f32 * btn_w + (row.len() as f32 - 1.0) * 8.0;
                        let pad = (total_row_w - row_w) / 2.0;
                        if pad > 0.0 { ui.add_space(pad); }
                        for &k in row {
                            if self.render_key_button(ui, k, btn_w, btn_h).clicked() {
                                self.active_key = k.to_string();
                                self.edit_value = self.config.keys.get(k).cloned().unwrap_or_default();
                            }
                        }
                    });
                }
            });

            ui.add_space(30.0);

            // Thumb Area & Stick
            ui.vertical(|ui| {
                ui.label("Thumb Area & Stick");
                ui.add_space(5.0);

                let center_offset = btn_w * 2.0 + 16.0;

                ui.horizontal(|ui| {
                    ui.add_space(center_offset);
                    if self.render_key_button(ui, "STICK_UP", btn_w, 38.0).clicked() {
                        self.active_key = "STICK_UP".to_string();
                        self.edit_value = self.config.keys.get("STICK_UP").cloned().unwrap_or_default();
                    }
                });

                ui.horizontal(|ui| {
                    if self.render_key_button(ui, "EXTRA1", btn_w, 38.0).clicked() {
                        self.active_key = "EXTRA1".to_string();
                        self.edit_value = self.config.keys.get("EXTRA1").cloned().unwrap_or_default();
                    }
                    if self.render_key_button(ui, "STICK_LEFT", btn_w, 38.0).clicked() {
                        self.active_key = "STICK_LEFT".to_string();
                        self.edit_value = self.config.keys.get("STICK_LEFT").cloned().unwrap_or_default();
                    }
                    if self.render_key_button(ui, "STICK_CLICK", btn_w, 38.0).clicked() {
                        self.active_key = "STICK_CLICK".to_string();
                        self.edit_value = self.config.keys.get("STICK_CLICK").cloned().unwrap_or_default();
                    }
                    if self.render_key_button(ui, "STICK_RIGHT", btn_w, 38.0).clicked() {
                        self.active_key = "STICK_RIGHT".to_string();
                        self.edit_value = self.config.keys.get("STICK_RIGHT").cloned().unwrap_or_default();
                    }
                });

                ui.horizontal(|ui| {
                    ui.add_space(center_offset);
                    if self.render_key_button(ui, "STICK_DOWN", btn_w, 38.0).clicked() {
                        self.active_key = "STICK_DOWN".to_string();
                        self.edit_value = self.config.keys.get("STICK_DOWN").cloned().unwrap_or_default();
                    }
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(center_offset);
                    if self.render_key_button(ui, "EXTRA2", btn_w, 38.0).clicked() {
                        self.active_key = "EXTRA2".to_string();
                        self.edit_value = self.config.keys.get("EXTRA2").cloned().unwrap_or_default();
                    }
                });
            });
        });

        if !self.active_key.is_empty() {
            ui.separator();
            ui.group(|ui| {
                ui.label(format!("Configuring Key: {}", self.active_key));
                ui.horizontal(|ui| {
                    ui.label("Keycode / Binding:");
                    ui.text_edit_singleline(&mut self.edit_value);
                    if ui.button("Save Binding").clicked() {
                        self.config.keys.insert(self.active_key.clone(), self.edit_value.clone());
                        self.save_current();
                        self.active_key.clear();
                    }

                    if ui.button("Clear Binding").clicked() {
                        self.config.keys.remove(&self.active_key);
                        self.save_current();
                        self.active_key.clear();
                    }

                    if !self.is_macro_recording {
                        if ui.button("Record Macro").clicked() {
                            self.is_macro_recording = true;
                            self.macro_target_key = self.active_key.clone();
                            self.recorded_macro_steps.clear();
                        }
                    } else {
                        if ui.button("Stop & Save Macro").clicked() {
                            self.is_macro_recording = false;
                            let macro_str = self.recorded_macro_steps.join(",");
                            self.edit_value = format!("MACRO:{}", macro_str);
                            self.config.keys.insert(self.macro_target_key.clone(), self.edit_value.clone());
                            self.save_current();
                            self.active_key.clear();
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.is_macro_recording = false;
                        self.active_key.clear();
                    }
                });

                if self.is_macro_recording {
                    ui.colored_label(egui::Color32::YELLOW, format!("Recording macro for [{}]. Press keys on G13 hardware...", self.macro_target_key));
                    ui.label(format!("Recorded Steps: [{}]", self.recorded_macro_steps.join(" -> ")));
                }
            });
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Max), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Apply & Save Profile").clicked() {
                    self.save_current();
                }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([920.0, 680.0]),
        ..Default::default()
    };
    eframe::run_native("G13 Nexus", options, Box::new(|_cc| Ok(Box::new(G13App::default()))))
}
