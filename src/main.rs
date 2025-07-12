use eframe::{self, App};
use egui::{self, FontFamily, RichText, TextStyle, ViewportBuilder};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::{collections::HashMap, fs, sync::Mutex};
use windows::Win32::Foundation::{HMODULE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_KEYBOARD, INPUT_TYPE, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, SendInput,
    VK_BACK, VK_CONTROL, VK_SPACE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, SetWindowsHookExA, UnhookWindowsHookEx, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

#[derive(Serialize, Deserialize, Clone)]
struct KeyboardSettings {
    enabled: bool,
    layout: String,
    current_language: String,
    use_suggestions: bool,
    hotkey_enabled: bool,
    font_size: f32,
    theme: String,
    intercept_all: bool,
}

#[derive(Clone)]
enum BanglaChar {
    Vowel(&'static str),
    Consonant(&'static str),
    VowelSign(&'static str),
    Number(&'static str),
    Special(&'static str),
}

// Global state
use std::sync::atomic;
lazy_static! {
    static ref CTRL_PRESSED: atomic::AtomicBool = atomic::AtomicBool::new(false);
    static ref KEYBOARD_HOOK: Mutex<Option<HHOOK>> = Mutex::new(None);
    static ref BUFFER: Mutex<String> = Mutex::new(String::new());
    static ref SETTINGS: Mutex<KeyboardSettings> = Mutex::new(KeyboardSettings {
        enabled: true,
        layout: "Phonetic".to_string(),
        current_language: "Bangla".to_string(),
        use_suggestions: true,
        hotkey_enabled: true,
        font_size: 14.0,
        theme: "Light".to_string(),
        intercept_all: true,
    });

    static ref PHONETIC_MAP: HashMap<&'static str, BanglaChar> = {
        let mut m = HashMap::new();

        // Vowels (স্বরবর্ণ)
        m.insert("a", BanglaChar::Vowel("অ"));
        m.insert("aa", BanglaChar::Vowel("আ"));
        m.insert("A", BanglaChar::Vowel("আ"));
        m.insert("i", BanglaChar::Vowel("ই"));
        m.insert("ii", BanglaChar::Vowel("ঈ"));
        m.insert("I", BanglaChar::Vowel("ঈ"));
        m.insert("u", BanglaChar::Vowel("উ"));
        m.insert("uu", BanglaChar::Vowel("ঊ"));
        m.insert("U", BanglaChar::Vowel("ঊ"));
        m.insert("rri", BanglaChar::Vowel("ঋ"));
        m.insert("e", BanglaChar::Vowel("এ"));
        m.insert("E", BanglaChar::VowelSign("ে"));
        m.insert("oi", BanglaChar::Vowel("ঐ"));
        m.insert("OI", BanglaChar::Vowel("ঐ"));
        m.insert("o", BanglaChar::Vowel("ও"));
        m.insert("O", BanglaChar::VowelSign("ো"));
        m.insert("ou", BanglaChar::Vowel("ঔ"));
        m.insert("OU", BanglaChar::Vowel("ঔ"));

        // Consonants (ব্যঞ্জনবর্ণ)
        m.insert("k", BanglaChar::Consonant("ক"));
        m.insert("kh", BanglaChar::Consonant("খ"));
        m.insert("g", BanglaChar::Consonant("গ"));
        m.insert("gh", BanglaChar::Consonant("ঘ"));
        m.insert("ng", BanglaChar::Consonant("ঙ"));
        m.insert("c", BanglaChar::Consonant("চ"));
        m.insert("ch", BanglaChar::Consonant("ছ"));
        m.insert("j", BanglaChar::Consonant("জ"));
        m.insert("jh", BanglaChar::Consonant("ঝ"));
        m.insert("ny", BanglaChar::Consonant("ঞ"));
        m.insert("t", BanglaChar::Consonant("ট"));
        m.insert("th", BanglaChar::Consonant("ঠ"));
        m.insert("d", BanglaChar::Consonant("ড"));
        m.insert("dh", BanglaChar::Consonant("ঢ"));
        m.insert("n", BanglaChar::Consonant("ন"));
        m.insert("p", BanglaChar::Consonant("প"));
        m.insert("ph", BanglaChar::Consonant("ফ"));
        m.insert("f", BanglaChar::Consonant("ফ"));
        m.insert("b", BanglaChar::Consonant("ব"));
        m.insert("bh", BanglaChar::Consonant("ভ"));
        m.insert("v", BanglaChar::Consonant("ভ"));
        m.insert("m", BanglaChar::Consonant("ম"));
        m.insert("z", BanglaChar::Consonant("য"));
        m.insert("r", BanglaChar::Consonant("র"));
        m.insert("l", BanglaChar::Consonant("ল"));
        m.insert("sh", BanglaChar::Consonant("শ"));
        m.insert("s", BanglaChar::Consonant("স"));
        m.insert("h", BanglaChar::Consonant("হ"));
        m.insert("y", BanglaChar::Consonant("য়"));
        m.insert("kk", BanglaChar::Consonant("ক্ক"));
        m.insert("tt", BanglaChar::Consonant("ত্ত"));
        m.insert("nn", BanglaChar::Consonant("ন্ন"));

        // Vowel Signs (কার)
        m.insert("kar_aa", BanglaChar::VowelSign("া"));
        m.insert("kar_i", BanglaChar::VowelSign("ি"));
        m.insert("kar_ii", BanglaChar::VowelSign("ী"));
        m.insert("kar_u", BanglaChar::VowelSign("ু"));
        m.insert("kar_uu", BanglaChar::VowelSign("ূ"));
        m.insert("kar_e", BanglaChar::VowelSign("ে"));
        m.insert("kar_oi", BanglaChar::VowelSign("ৈ"));
        m.insert("kar_o", BanglaChar::VowelSign("ো"));
        m.insert("kar_ou", BanglaChar::VowelSign("ৌ"));

        // Numbers
        m.insert("0", BanglaChar::Number("০"));
        m.insert("1", BanglaChar::Number("১"));
        m.insert("2", BanglaChar::Number("২"));
        m.insert("3", BanglaChar::Number("৩"));
        m.insert("4", BanglaChar::Number("৪"));
        m.insert("5", BanglaChar::Number("৫"));
        m.insert("6", BanglaChar::Number("৬"));
        m.insert("7", BanglaChar::Number("৭"));
        m.insert("8", BanglaChar::Number("৮"));
        m.insert("9", BanglaChar::Number("৯"));

        // Special Characters
        m.insert("chandrabindu", BanglaChar::Special("ঁ"));
        m.insert("anusvar", BanglaChar::Special("ং"));
        m.insert("bisarga", BanglaChar::Special("ঃ"));
        m.insert("hasant", BanglaChar::Special("্"));
        m.insert("dari", BanglaChar::Special("।"));

        m
    };

    static ref CONVERSION_MAP: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        // Convert PHONETIC_MAP to simple string mappings for display
        for (k, v) in PHONETIC_MAP.iter() {
            match v {
                BanglaChar::Vowel(c) | BanglaChar::Consonant(c) |
                BanglaChar::VowelSign(c) | BanglaChar::Number(c) |
                BanglaChar::Special(c) => {
                    m.insert(*k, *c);
                }
            }
        }
        m
    };
}

struct KeyboardApp {
    show_settings: bool,
    suggestions: Vec<String>,
    search_text: String,
    selected_category: String,
}

impl Default for KeyboardApp {
    fn default() -> Self {
        Self {
            show_settings: false,
            suggestions: Vec::new(),
            search_text: String::new(),
            selected_category: "All".to_string(),
        }
    }
}

impl KeyboardApp {
    fn update_suggestions(&mut self) {
        self.suggestions.clear();
        if self.search_text.is_empty() {
            return;
        }

        for (eng, bang) in CONVERSION_MAP.iter() {
            if eng.contains(&self.search_text.to_lowercase()) {
                self.suggestions.push(format!("{} → {}", eng, bang));
            }
        }
    }

    fn matches_category(&self, key: &str) -> bool {
        match self.selected_category.as_str() {
            "All" => true,
            "Vowels" => PHONETIC_MAP
                .get(key)
                .map_or(false, |c| matches!(c, BanglaChar::Vowel(_))),
            "Consonants" => PHONETIC_MAP
                .get(key)
                .map_or(false, |c| matches!(c, BanglaChar::Consonant(_))),
            "Numbers" => PHONETIC_MAP
                .get(key)
                .map_or(false, |c| matches!(c, BanglaChar::Number(_))),
            "Special" => PHONETIC_MAP
                .get(key)
                .map_or(false, |c| matches!(c, BanglaChar::Special(_))),
            _ => false,
        }
    }

    fn get_font_size(&self) -> f32 {
        SETTINGS.lock().unwrap().font_size
    }
}

impl App for KeyboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Settings").clicked() {
                        self.show_settings = true;
                    }
                    if ui.button("Exit").clicked() {
                        std::process::exit(0);
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        // Show about dialog
                    }
                });

                // Keyboard status and language indicators
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let settings = SETTINGS.lock().unwrap();
                    let enabled = settings.enabled;
                    let is_bangla = settings.current_language == "Bangla";

                    ui.horizontal(|ui| {
                        // Modern language indicator
                        ui.label(
                            RichText::new(if is_bangla { "বাংলা" } else { "En" })
                                .size(20.0)
                                .color(if enabled {
                                    egui::Color32::from_rgb(0, 150, 0)
                                } else {
                                    egui::Color32::GRAY
                                }),
                        );

                        // Keyboard shortcut hint
                        ui.label(RichText::new("(Ctrl+Space)").weak().size(12.0));
                    });

                    ui.add_space(10.0);
                });
            });
        });

        // Settings window
        if self.show_settings {
            egui::Window::new("Settings")
                .open(&mut self.show_settings)
                .show(ctx, |ui| {
                    let mut settings = SETTINGS.lock().unwrap();
                    ui.vertical(|ui| {
                        // Enable/Disable keyboard
                        if ui
                            .checkbox(&mut settings.enabled, "Enable keyboard")
                            .clicked()
                        {
                            settings.enabled = !settings.enabled;
                        }

                        ui.add_space(10.0);

                        // Language selector
                        ui.horizontal(|ui| {
                            ui.label("Language:");
                            if ui
                                .radio_value(
                                    &mut settings.current_language,
                                    "Bangla".to_string(),
                                    "বাংলা",
                                )
                                .clicked()
                            {
                                settings.enabled = true;
                            }
                            if ui
                                .radio_value(
                                    &mut settings.current_language,
                                    "English".to_string(),
                                    "English",
                                )
                                .clicked()
                            {
                                settings.enabled = false;
                            }
                        });

                        ui.add_space(10.0);

                        // Font size
                        ui.horizontal(|ui| {
                            ui.label("Font Size:");
                            ui.add(
                                egui::Slider::new(&mut settings.font_size, 12.0..=24.0)
                                    .step_by(1.0),
                            );
                        });

                        ui.add_space(10.0);

                        // Theme
                        ui.horizontal(|ui| {
                            ui.label("Theme:");
                            ui.radio_value(&mut settings.theme, "Light".to_string(), "Light");
                            ui.radio_value(&mut settings.theme, "Dark".to_string(), "Dark");
                        });

                        ui.add_space(10.0);

                        // Additional settings
                        ui.checkbox(&mut settings.use_suggestions, "Show typing suggestions");
                        ui.checkbox(&mut settings.hotkey_enabled, "Enable Ctrl+Space shortcut");
                    });
                });
        }

        // Layout preview
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Keyboard Layout Preview");
                ui.separator();
                // Search box
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let search_response = ui.text_edit_singleline(&mut self.search_text);
                    ui.label("Search: ");
                    if search_response.changed() {
                        self.update_suggestions();
                    }
                });
            });

            ui.add_space(10.0);

            // Category selector
            ui.horizontal(|ui| {
                ui.label("Category: ");
                egui::ComboBox::from_label("")
                    .selected_text(&self.selected_category)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.selected_category, "All".to_string(), "All");
                        ui.selectable_value(
                            &mut self.selected_category,
                            "Vowels".to_string(),
                            "Vowels",
                        );
                        ui.selectable_value(
                            &mut self.selected_category,
                            "Consonants".to_string(),
                            "Consonants",
                        );
                        ui.selectable_value(
                            &mut self.selected_category,
                            "Numbers".to_string(),
                            "Numbers",
                        );
                        ui.selectable_value(
                            &mut self.selected_category,
                            "Special".to_string(),
                            "Special",
                        );
                    });
            });

            ui.add_space(10.0);

            // Split view for mappings and suggestions
            ui.columns(2, |columns| {
                // Left column: Mappings
                columns[0].group(|ui| {
                    ui.set_min_height(400.0);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let mut col_counter = 0;
                        egui::Grid::new("keyboard_layout")
                            .spacing([10.0, 10.0])
                            .show(ui, |ui| {
                                for (eng, bang) in CONVERSION_MAP.iter().filter(|(k, _)| {
                                    self.search_text.is_empty()
                                        || k.contains(&self.search_text.to_lowercase())
                                }) {
                                    if self.matches_category(eng) {
                                        ui.horizontal(|ui| {
                                            // English input text
                                            ui.label(
                                                RichText::new(*eng)
                                                    .text_style(TextStyle::Body)
                                                    .monospace(),
                                            );

                                            // Arrow with some spacing
                                            ui.add_space(5.0);
                                            ui.label(
                                                RichText::new("→")
                                                    .text_style(TextStyle::Body)
                                                    .color(egui::Color32::GRAY),
                                            );
                                            ui.add_space(5.0);

                                            // Bengali output text
                                            ui.label(
                                                RichText::new(*bang)
                                                    .size(self.get_font_size())
                                                    .strong()
                                                    .color(egui::Color32::from_rgb(0, 100, 0)),
                                            );
                                        });
                                        col_counter += 1;
                                        if col_counter % 2 == 0 {
                                            ui.end_row();
                                        }
                                    }
                                }
                                if col_counter % 2 != 0 {
                                    ui.end_row();
                                }
                            });
                    });
                });

                // Right column: Suggestions
                columns[1].group(|ui| {
                    ui.set_min_height(400.0);
                    ui.heading("Suggestions");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for suggestion in &self.suggestions {
                            ui.label(suggestion);
                        }
                    });
                });
            });
        });
    }
}

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let kbd_struct = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
    let vk_code = kbd_struct.vkCode;
    let flags = kbd_struct.flags;

    println!(
        "Key event: code={:x}, type={}, flags={:x}",
        vk_code, wparam.0, flags.0
    );

    if code < 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    // Don't process injected keystrokes (prevents infinite recursion)
    if (flags & windows::Win32::UI::WindowsAndMessaging::KBDLLHOOKSTRUCT_FLAGS(0x10)).0 != 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    // Print debug info
    println!(
        "Key: {:x}, Type: {}, Ctrl: {}",
        vk_code,
        wparam.0,
        CTRL_PRESSED.load(Ordering::SeqCst)
    );

    let msg_type = wparam.0 as u32;
    println!(
        "Key event: code={:x}, type={}, injected={}",
        vk_code,
        msg_type,
        (flags.0 & 0x10) != 0
    );

    match msg_type {
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            if vk_code == VK_CONTROL.0 as u32 {
                CTRL_PRESSED.store(true, Ordering::SeqCst);
            }

            // Handle backspace
            if vk_code == VK_BACK.0 as u32 {
                let mut buffer = BUFFER.lock().unwrap();
                if !buffer.is_empty() {
                    buffer.pop();
                    println!("Backspace pressed, buffer now: {}", buffer);
                }
                return unsafe { CallNextHookEx(None, code, wparam, lparam) };
            }

            let settings = SETTINGS.lock().unwrap();
            if settings.enabled {
                // Handle language switching hotkey (Ctrl+Space)
                if settings.hotkey_enabled {
                    if vk_code == VK_SPACE.0 as u32 && CTRL_PRESSED.load(Ordering::SeqCst) {
                        drop(settings); // Release lock before modifying
                        let mut settings = SETTINGS.lock().unwrap();
                        let new_lang = if settings.current_language == "Bangla" {
                            "English"
                        } else {
                            "Bangla"
                        };
                        settings.current_language = new_lang.to_string();
                        return LRESULT(1);
                    }
                }

                // Process key input if in Bangla mode
                if settings.current_language == "Bangla" && settings.intercept_all {
                    let key = if vk_code >= 0x41 && vk_code <= 0x5A {
                        // Convert A-Z to lowercase a-z
                        Some(((vk_code - 0x41 + 0x61) as u8 as char).to_string())
                    } else if vk_code >= 0x30 && vk_code <= 0x39 {
                        // Numbers 0-9
                        Some(((vk_code - 0x30) as u8 as char).to_string())
                    } else {
                        None
                    };

                    if let Some(key) = key {
                        println!("Detected key: {}", key);
                        let mut buffer = BUFFER.lock().unwrap();

                        // If this is a vowel and the buffer is empty, handle it directly
                        if buffer.is_empty() && matches!(key.as_str(), "a" | "e" | "i" | "o" | "u")
                        {
                            if let Some(bangla_char) = PHONETIC_MAP.get(key.as_str()) {
                                if let BanglaChar::Vowel(c) = bangla_char {
                                    simulate_unicode_input(c);
                                    return LRESULT(1);
                                }
                            }
                        }

                        if let Some((output, backspaces)) =
                            process_keyboard_input(&key, &mut buffer)
                        {
                            println!(
                                "Processing result: output='{}', backspaces={}",
                                output, backspaces
                            );
                            drop(buffer); // Release lock before simulating input

                            // First remove the typed English text
                            for _ in 0..backspaces {
                                simulate_backspace();
                                std::thread::sleep(std::time::Duration::from_millis(5));
                            }

                            // Then send the Bangla text
                            if !output.is_empty() {
                                std::thread::sleep(std::time::Duration::from_millis(5));
                                simulate_unicode_input(&output);
                            }
                            return LRESULT(1);
                        }
                    }
                }
            }
        }
        WM_KEYUP | WM_SYSKEYUP => {
            if vk_code == VK_CONTROL.0 as u32 {
                CTRL_PRESSED.store(false, Ordering::SeqCst);
            }
        }
        _ => {}
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up keyboard hook first
    unsafe {
        *KEYBOARD_HOOK.lock().unwrap() = Some(SetWindowsHookExA(
            WH_KEYBOARD_LL,
            Some(keyboard_hook_proc),
            HMODULE(0),
            0,
        )?);
    }

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0])
            .with_title("Restro Keyboard"),
        follow_system_theme: true,
        default_theme: eframe::Theme::Light,
        ..Default::default()
    };

    // Try to load local Bengali font first, then fall back to system fonts
    let bengali_font_path = if std::path::Path::new("assets/fonts/Nirmala.ttf").exists() {
        "assets/fonts/Nirmala.ttf".to_string()
    } else {
        std::env::var("WINDIR")
            .map(|windir| {
                let font_paths = [
                    format!("{}\\Fonts\\Nirmala.ttf", windir),
                    format!("{}\\Fonts\\Vrinda.ttf", windir),
                    format!("{}\\Fonts\\Shonar.ttf", windir),
                ];
                font_paths
                    .into_iter()
                    .find(|path| std::path::Path::new(path).exists())
            })
            .ok()
            .flatten()
            .ok_or_else(|| "No Bengali font found")?
    };

    // Load font data
    let font_data = fs::read(&bengali_font_path)?;

    // Initialize window for hook context
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExA, HWND_MESSAGE, WS_OVERLAPPED,
        };
        let window = CreateWindowExA(
            Default::default(),
            windows::s!("STATIC"),
            windows::s!("Restro Keyboard Hook"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            None,
            None,
        );
        if window.0 == 0 {
            return Err("Failed to create message-only window".into());
        }
    };

    // Run UI in the main thread
    eframe::run_native(
        "Restro Keyboard",
        options,
        Box::new(move |cc| {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "bengali".to_owned(),
                egui::FontData::from_owned(font_data.clone()),
            );

            for family in [FontFamily::Proportional, FontFamily::Monospace] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .insert(0, "bengali".to_owned());
            }

            cc.egui_ctx.set_fonts(fonts);
            Box::new(KeyboardApp::default())
        }),
    )?;

    // Clean up hook on exit
    unsafe {
        if let Some(hook) = KEYBOARD_HOOK.lock().unwrap().take() {
            UnhookWindowsHookEx(hook);
        }
    }

    Ok(())
}

fn process_keyboard_input(key: &str, buffer: &mut String) -> Option<(String, usize)> {
    buffer.push_str(key);
    let buffer_str = buffer.as_str();

    println!("Processing input - Buffer: {}, Key: {}", buffer_str, key);

    // Special case: if the buffer gets too long, clear it
    if buffer_str.len() > 5 {
        buffer.clear();
        return None;
    }

    // Try longer matches first (up to 3 characters)
    for len in (1..=std::cmp::min(buffer_str.len(), 3)).rev() {
        if let Some(substr) = buffer_str.get(buffer_str.len() - len..) {
            // Handle vowel signs after consonants
            if len == 1 {
                if let Some(prev) = buffer_str.chars().nth(buffer_str.len() - 2) {
                    if let Some(BanglaChar::Consonant(_)) =
                        PHONETIC_MAP.get(prev.to_string().as_str())
                    {
                        let result = match substr {
                            "a" => Some((String::new(), 1)), // Remove 'a' after consonant
                            "i" => Some(("ি".to_string(), 1)),
                            "e" => Some(("ে".to_string(), 1)),
                            "u" => Some(("ু".to_string(), 1)),
                            "o" => Some(("ো".to_string(), 1)),
                            _ => None,
                        };

                        if result.is_some() {
                            buffer.clear();
                            return result;
                        }
                    }
                }
            }

            // Try exact match for the current substring
            if let Some(bangla_char) = PHONETIC_MAP.get(substr) {
                println!("Found match for: {}", substr);

                let prev_was_consonant = if len < buffer_str.len() {
                    buffer_str
                        .chars()
                        .nth(buffer_str.len() - len - 1)
                        .map(|ch| {
                            PHONETIC_MAP
                                .get(ch.to_string().as_str())
                                .map(|bc| matches!(bc, BanglaChar::Consonant(_)))
                                .unwrap_or(false)
                        })
                        .unwrap_or(false)
                } else {
                    false
                };

                let output = match bangla_char {
                    BanglaChar::Consonant(c) => {
                        if prev_was_consonant {
                            format!("্{}", c)
                        } else {
                            c.to_string()
                        }
                    }
                    BanglaChar::VowelSign(c) => c.to_string(),
                    BanglaChar::Vowel(c) => {
                        if prev_was_consonant {
                            match *c {
                                "অ" => String::new(), // Remove 'a' after consonant
                                "আ" => "া".to_string(),
                                "ই" => "ি".to_string(),
                                "ঈ" => "ী".to_string(),
                                "উ" => "ু".to_string(),
                                "ঊ" => "ূ".to_string(),
                                "এ" => "ে".to_string(),
                                "ঐ" => "ৈ".to_string(),
                                "ও" => "ো".to_string(),
                                "ঔ" => "ৌ".to_string(),
                                _ => c.to_string(),
                            }
                        } else {
                            c.to_string()
                        }
                    }
                    BanglaChar::Number(c) | BanglaChar::Special(c) => c.to_string(),
                };

                buffer.clear();
                return Some((output, len));
            }
        }
    }

    None
}

fn simulate_backspace() {
    unsafe {
        let input1 = INPUT {
            r#type: INPUT_TYPE(INPUT_KEYBOARD.0),
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_BACK,
                    wScan: 0,
                    dwFlags: Default::default(),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let input2 = INPUT {
            r#type: INPUT_TYPE(INPUT_KEYBOARD.0),
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_BACK,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let inputs = [input1, input2];
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

fn simulate_unicode_input(text: &str) {
    // Small delay between characters to ensure reliable input
    let delay = std::time::Duration::from_millis(1);

    for c in text.chars() {
        unsafe {
            let input1 = INPUT {
                r#type: INPUT_TYPE(INPUT_KEYBOARD.0),
                Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: Default::default(),
                        wScan: c as u16,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };

            let input2 = INPUT {
                r#type: INPUT_TYPE(INPUT_KEYBOARD.0),
                Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: Default::default(),
                        wScan: c as u16,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };

            let inputs = [input1, input2];
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);

            // Small delay to ensure characters are typed in the correct order
            std::thread::sleep(delay);
        }
    }
}
