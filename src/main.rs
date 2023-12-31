#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

pub mod ankiconnect;
pub mod data_sources;

use egui::TextBuffer;
use regex::Regex;

use eframe::egui;
use arboard::Clipboard;

struct CardCreator {
    word: String,
    sentence: String,
    texture: Option<egui::TextureHandle>,
    image: Option<egui::ColorImage>,
    raw_rgba_image: Option<image::RgbaImage>,
    clipboard: Clipboard,
    logs: String,
    anki_client: ankiconnect::AnkiClient,
    kanjiapi_client: data_sources::kanjiapi::KanjiAPIClient,
    jisho_client: data_sources::jisho::JishoClient,
    decks: Vec<String>,
    selected_deck: String,
    word_cleaner_regex: Regex,
    current_dir: String,
    tokio_rt: tokio::runtime::Runtime
}

impl CardCreator {
    fn new(cc: &eframe::CreationContext<'_>, tokio_rt: tokio::runtime::Runtime) -> Self {
        cc.egui_ctx.set_pixels_per_point(1.5);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        
        let anki_client = ankiconnect::AnkiClient::new();
        let clipboard = Clipboard::new().unwrap();

        let mut decks: Vec<String> = vec![];
        let mut logs = String::new();

        let get_anki_decks = anki_client.get_decks();

        match get_anki_decks {
            Ok(anki_decks) => {
                decks = anki_decks;
            },
            Err(error) => {
                logs.push_str(format!("Anki is not connected. Raw error: {}\n", error.to_string()).as_str());
            }
        }

        setup_custom_fonts(&cc.egui_ctx);

        let word_cleaner_regex = Regex::new(r"[^\p{Han}\p{Hiragana}\p{Katakana}]+").unwrap();
        let current_dir = std::env::current_dir().unwrap().to_string_lossy().take();

        Self {
            word: String::new(),
            sentence: String::new(),
            texture: None,
            image: None,
            raw_rgba_image: None,
            clipboard,
            logs,
            anki_client,
            decks,
            selected_deck: String::from("Select deck"),
            jisho_client: data_sources::jisho::JishoClient::new(),
            kanjiapi_client: data_sources::kanjiapi::KanjiAPIClient::new(),
            word_cleaner_regex,
            current_dir,
            tokio_rt
        }
    }

}

impl eframe::App for CardCreator {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {

            // Load the texture from the image saved on the clipboard if exists
            if let Some(saved_image) = self.image.clone() {
                self.texture = Some(
                    ui.ctx().load_texture("screenshot", saved_image, Default::default())
                );
            }
            
            // Dealing with Ctrl V event stuff
            if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
                if let Ok(img) = self.clipboard.get_image() {
                    let _image: image::RgbaImage = image::ImageBuffer::from_raw(
                        img.width.try_into().unwrap(),
                        img.height.try_into().unwrap(),
                        img.bytes.clone().into_owned()
                    ).unwrap();
                    let size = [_image.width() as _, _image.height() as _];

                    self.image = Some(egui::ColorImage::from_rgba_unmultiplied(size, _image.as_flat_samples().as_slice()));
                    self.raw_rgba_image = Some(_image);
                    return;
                }

                self.logs.push_str("No picture found on the clipboard\n");
            }

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
    
                    ui.heading("Word");
                    let text_edit = egui::TextEdit::singleline(&mut self.word).hint_text("insert word to add...").desired_width(600.0);
                    ui.add(text_edit);
   
                    ui.add_space(10.0);

                    ui.heading("Sentence");
                    let text_edit = egui::TextEdit::singleline(&mut self.sentence).hint_text("insert sentence to add...").desired_width(600.0);
                    ui.add(text_edit);

                    ui.add_space(10.0);

                    ui.heading("Deck");

                    let combo = egui::ComboBox::from_id_source(2312).width(600.0);
                    
                    combo.selected_text(self.selected_deck.to_owned()).show_ui(ui, |ui| {
                        for _deck in self.decks.iter() {
                            ui.selectable_value(&mut self.selected_deck, _deck.to_owned(), _deck.to_owned());
                        }
                    });

                    ui.add_space(20.0);

                    let refresh_decks_button = egui::Button::new("Refresh deck list").min_size(egui::Vec2 { x: 200.0, y: 25.0}).shortcut_text("Refresh decks");

                    if ui.add(refresh_decks_button).clicked() {
                        let get_anki_decks = self.anki_client.get_decks();

                        match get_anki_decks {
                            Ok(anki_decks) => {
                                self.decks = anki_decks;
                                self.logs.push_str("Deck list refreshed!\n");
                            },
                            Err(error) => {
                                self.logs.push_str(format!("Anki is not connected. Raw error: {}\n", error.to_string()).as_str());
                            }
                        }
                    }

                    ui.add_space(20.0);

                    let add_note_button = egui::Button::new("Add note").min_size(egui::Vec2 { x: 200.0, y: 25.0}).shortcut_text("Add note");
                    
                    if ui.add(add_note_button).clicked() {

                        if !self.decks.contains(&self.selected_deck) {
                            self.logs.push_str("No deck selected\n");
                            return;
                        }
                        let now = std::time::Instant::now();
                        
                        self.word = self.word_cleaner_regex.replace_all(&self.word, "").to_string();

                        let _card = ankiconnect::create_card(
                            &self.tokio_rt,
                            &self.jisho_client,
                            &self.kanjiapi_client,
                            &self.current_dir,
                            &self.word,
                            &self.sentence,
                            &self.raw_rgba_image,
                            &self.selected_deck,
                        );

                        if let Err(card_error) = _card {
                            self.logs.push_str(format!("{:?}\n", card_error.as_str()).as_str());
                            return;
                        }

                        let anki_response = self.anki_client.add_card_to_deck(&_card.unwrap());
                        let elapsed = now.elapsed();

                        match anki_response {
                            Ok(_) => {
                                self.logs.push_str(format!("Added word {} to deck {} [{:?}]\n", self.word, self.selected_deck, elapsed).as_str());
                            },
                            Err(_error) => {
                                self.logs.push_str(format!("{:?}\n", _error).as_str());
                            }
                        }

                    }

                    ui.add_space(20.0);

                    ui.heading("Logs:");

                    egui::ScrollArea::vertical()
                        .max_width(590.0)
                        .min_scrolled_height(280.0)
                        .auto_shrink([true, true])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.label(&self.logs);
                        });

                });
                
                ui.vertical(|ui| {
                    ui.heading("Picture: (Press F5 to paste picture from clipboard)");

                    // Needs to use as_ref because otherwise the if will take ownership
                    // and the texture will be cleaned up
                    // Actually using clone() would work too
                    if let Some(texture) = self.texture.as_ref() {
                        ui.image((texture.id(), eframe::egui::Vec2 { x: 750.0, y: 590.0 }));
                    }
                });
                ui.ctx().request_repaint();
            });
        });
    }

}

fn main() -> Result<(), eframe::Error> {

    let tokio_rt = tokio::runtime::Runtime::new().unwrap();
    let _enter_guard = tokio_rt.enter();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1600.0, 800.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Anki Japanese Card Creator",
        options,
        Box::new(|cc| {
            Box::new(CardCreator::new(&cc, tokio_rt))
        }),
    )
}

// Copied from custom_fonts.rs example
fn setup_custom_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();

    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    fonts.font_data.insert(
        "meiryo".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../meiryo.ttf"
        )),
    );

    // Put my font first (highest priority) for proportional text:
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "meiryo".to_owned());

    // Put my font as last fallback for monospace:
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("meiryo".to_owned());

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}
