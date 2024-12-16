use eframe::{egui, run_native, NativeOptions};
use uuid::Uuid;
use egui::epaint::text;
use regex::Regex;

#[derive(serde::Serialize, serde:: Deserialize, Clone)]
struct Card {
    id: uuid::Uuid,
    name: String,
    description: String,
    url: String,
    tags: Vec<String>,
    folders: Vec<String>,
}

impl Default for Card{
    fn default() -> Self {
         Card{
            id: uuid::Uuid::new_v4(),
            name: String::default(),
            description: String::default(),
            url: String::default(),
            tags: Vec::default(),
            folders: Vec::default(),
         }
    }
}

#[derive(Default, Debug, Clone)]
struct Layout {
    full_height: f32,
    full_width: f32,
    input_station_height: f32,
    input_station_input_height: f32,
    input_station_output_height: f32,
    input_station_width: f32,
    input_station_right_margin: f32,
    card_height: f32,
    card_width: f32,
}

fn main() {
    let native_options = NativeOptions::default();
    run_native("TRONK", native_options, Box::new(|cc| Ok(Box::new(TRONK::new(cc)))));
}

#[derive(Default)]
struct TRONK {
    label: String,
    cards: Vec<Card>,
    system_output_text: String,
    user_input_text: String,
    init_input_station_pos: egui::Pos2,
    is_input_station_open: bool,
    folders: Vec<String>,
    undo_stack: Vec<Vec<Card>>,
    selected_card: Option<usize>,
    detailed_card: Option<Card>,
}

impl TRONK {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.load_cards();
        app.is_input_station_open = true;
        app
    }
    fn load_cards(&mut self){
        match std::fs::File::open("data.json"){
            Ok(file) => {
                let reader = std::io::BufReader::new(file);
                match serde_json::from_reader(reader){
                    Ok(cards) => self.cards = cards,
                    Err(e) => println!("error loading cards, starting with empty set, {}", e),
                }
            }
            Err(e) => println!("file not found, starting with emptry sets {}", e),
        }
    }
    fn save_cards(&self){
        let file = std::fs::File::create("cards.json").unwrap();
        let writer = std::io::BufWriter::new(file);
         serde_json::to_writer_pretty(writer, &self.cards).unwrap();
    }
    fn calculate_layout(&self, width:f32, height:f32) -> Layout{
        let full_width = width;
        let full_height= height;
        Layout { 
            full_width: full_width,
            full_height: full_height,
            input_station_height: full_height/3.0,
            input_station_width: 200.0,
            input_station_right_margin: 50.0,
            input_station_input_height: 50.0,
            input_station_output_height : 150.0,
            card_height: 120.0,
            card_width: 180.0,
        }
    }
    fn process_input(&mut self) {
             // TODO: integrate OLLAMA
        let input = self.user_input_text.trim();
        self.system_output_text= format!("command: {}, is being processed...", input);
            if input.starts_with("/add"){
            let re = Regex::new(r"^(?P<name>[^#@]+)(?:\s*#(?P<tags>[^@]+))?(?:\s*@(?P<folders>.+))?$").unwrap();
            match re.captures(input.trim_start_matches("/add").trim()) {
                    Some(captures) => {
                    let name = captures.name("name").map_or("", |m| m.as_str()).trim().to_string();
                        let tags_str = captures.name("tags").map_or("", |m| m.as_str());
                        let folders_str = captures.name("folders").map_or("", |m| m.as_str());
                        let tags: Vec<String> = tags_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                        let folders: Vec<String> = folders_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

                        if name.is_empty(){
                            self.system_output_text = "card name cannot be empty".to_string();
                        } else {
                            let mut new_card = Card::default();
                            new_card.name = name;
                            new_card.description = "".to_string();
                            new_card.url = "".to_string();
                            new_card.tags = tags;
                            new_card.folders = folders;
                        self.system_output_text = format!("card created with name: {}", new_card.name);
                        self.undo_stack.push(self.cards.clone()); // <-- add this to undo stack
                        self.cards.push(new_card);
                        };
                    },
                    None => {
                        self.system_output_text = "invalid card format. try: '/add cardname #tag1,tag2 @folder1,folder2'".to_string();
                    }
                }

        } else  if input.starts_with("/ollama") {
            let prompt = input.split_once(" ").map(|(_, s)| s).unwrap_or("").trim();
                if prompt.is_empty() {
                    self.system_output_text = "no prompt provided".to_string();
                    self.user_input_text= "".to_string();
                    return;
                }
                let output = self.ollama_inference(prompt);
                self.system_output_text= output.clone();
                self.user_input_text = "".to_string();
        } else {
            self.system_output_text = format!("command: {}, is not implemented", input);
        }

        self.user_input_text= "".to_string();
        }
        fn ollama_inference(&self, prompt: &str) -> String{
            let output = std::process::Command::new("ollama")
                .arg("run")
                .arg("tinyllama")
                .arg(prompt)
                .output()
                .expect("failed to execute ollama command");
            if output.status.success(){
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                String::from_utf8_lossy(&output.stderr).to_string()
        }
        }
}

impl eframe::App for TRONK {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let screen_width = ctx.screen_rect().width();
        let screen_height = ctx.screen_rect().height();
        let layout = self.calculate_layout(screen_width, screen_height);
        let num_cards = self.cards.len();
        let card_width = layout.card_width;
        let card_spacing = layout.card_height;
        let columns = (layout.full_width/ (card_width + card_spacing)).floor() as usize;
        
        // Ensure selected_card has a valid value
        if let Some(index) = self.selected_card {
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                if index >= columns {
                    self.selected_card = Some(index - columns); // Move to the card above
                }
            } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                if index + columns < num_cards {
                    self.selected_card = Some(index + columns); // Move to the card below
                }
            } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                if index > 0 {
                    self.selected_card = Some(index - 1); // Move to the left
                }
            } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                if index + 1 < num_cards {
                    self.selected_card = Some(index + 1); // Move to the right
                }
            } else if ctx.input(|i| i.key_pressed(egui::Key::O)) {
                // CHANGEHERE: Set the highlighted card as the selected card on pressing Enter
                if let Some(selected) = self.selected_card {
                    self.detailed_card = Some(self.cards[selected].clone());
                    self.system_output_text = format!("Selected card: {}", self.cards[selected].name);
                }
            }
        }
        else if num_cards > 0 {
            // Default to selecting the first card
            self.selected_card = Some(0);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Slash)){
            if !self.is_input_station_open{
                self.is_input_station_open = true;
                //TODO UNCOLLAPSE INPUT STATION WINDOW
            }
        }
        //CARD VIEW
       egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_height(layout.full_height);
            ui.set_width(layout.full_width);
            let frame = egui::Frame {
                // fill: egui::Color32::,
                // stroke: egui::Stroke::new(0.5, egui::Color32::DARK_BLUE),
                inner_margin: egui::Margin::same(10.0),
                outer_margin: egui::Margin::same(0.0),
                ..Default::default()
            };
            frame.show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui|{
                    ui.set_width(layout.full_width - 35.0);
                    ui.set_height(layout.full_height);
                    ui.horizontal_wrapped(|ui| {
                        let mut card_index = 0;
                        for card in &self.cards{
                            let is_selected = self.selected_card.map_or(false, |i| i == card_index);
                            let card_spacing = 10.0; // Spacing between cards
                            let available_width = ui.available_width(); // Dynamic parent width
                            let columns = (available_width / (card_width + card_spacing)).floor() as usize; // Calculate number of columns
                                    // let card_response = ui.allocate_rect(
                                    //     egui::Rect::from_min_size(
                                    //         ui.cursor().min,
                                    //         egui::vec2(110.0, 90.0), // Card dimensions
                                    //     ),
                                    //     egui::Sense::click(),
                                    // );
                                    let card_rect = ui.cursor();
                                    // let card_rect = ui.allocate_space(egui::vec2(110.0, 90.0)); // Reserve space for the card
                                    let card_size = egui::vec2(110.0, 90.0);
                                    let card_response = ui.allocate_rect(
                                        egui::Rect::from_min_size(card_rect.min, card_size),
                                        egui::Sense::click(),
                                    );
                                    let card_frame = egui::Frame {
                                        // fill: egui::Color32::WHITE,
                                        stroke: if is_selected {
                                            egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE)
                                        } else {
                                            egui::Stroke::new(1.0, egui::Color32::DARK_BLUE)
                                        },
                                    ..Default::default()
                                    };
                                    
                                    card_frame.show(ui, |ui| {
                                        ui.set_width(110.0);
                                        ui.set_height(90.0);
                                        ui.vertical(|ui|{
                                        ui.horizontal_top(|ui| {
                                                 ui.set_height(65.0);
                                                ui.label("img");
                                            });
                                        ui.separator();
                                        ui.horizontal(|ui|{
                                                ui.set_height(15.0);
                                                let mut name = card.name.clone();
                                                if name.len() > 10{
                                                    name.truncate(10);
                                                    name.push_str("...");
                                                }
                                                 ui.label(name);
                                            let tags_text = card.tags.iter().map(|tag| format!("#{}", tag)).collect::<Vec<_>>().join(" ");
                                          ui.horizontal(|ui|{
                                                ui.label(tags_text);
                                            })
                                                });
                                            });
                                     });
                                         if card_response.clicked(){
                                            self.system_output_text = format!("Clicked card: {}", card.name);
                                            self.detailed_card = Some(card.clone());
                                         }
                                    card_index += 1;
                                      // CHANGEHERE: Ensure cards wrap correctly to the next row
                                    if card_index % columns == 0 {
                                        ui.end_row();
                                    }
                                    }
                                });
             });
         });
        });
        //DETAILED VIEW
        let window_width = ctx.screen_rect().width();
        let window_height = ctx.screen_rect().height();

        if let Some(card) = &self.detailed_card {
                let mut is_open = true;
                egui::Window::new("Card Details")
                    .open(&mut is_open)
                    .fixed_pos(egui::pos2(window_width / 2.0, 0.0))
                    .default_width(window_width / 2.0)
                    .default_height(window_height)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.label("image preview");
                            ui.separator();
                                ui.label(format!("name: {}", card.name));
                                ui.separator();
                                ui.label(format!("tags: {}", card.tags.iter().map(|tag| format!("#{}", tag)).collect::<Vec<_>>().join(" ")));
                                ui.separator();
                                ui.label(format!("url: {}", card.url));
                                ui.separator();
                            ui.label(format!("description: {}", card.description));
                        });
                });

            if !is_open {
                self.detailed_card = None;
            }
       }
        //INPUT STATION WINDOW
        egui::Window::new("input station")
                .default_pos(egui::pos2(layout.full_width - layout.input_station_width - layout.input_station_right_margin, layout.full_height - layout.input_station_height)) //set position to bottom right
                .resizable(false)
                .collapsible(true)
                // .movable(true)
                .title_bar(true)
                .show(ctx, |ui| {
                    let frame = egui::Frame{
                    //    fill: egui::Color32::BLACK,
                       ..Default::default()
                   }.inner_margin(egui::Margin::same(8.0));
                    frame.show(ui, |ui| {
                        ui.set_width(layout.input_station_width);
                        ui.vertical(|ui|{
                        // system output
                         ui.vertical(|ui| {
                            ui.set_width(layout.input_station_width);
                            ui.set_height(layout.input_station_output_height);
                            ui.label(format!("system output: {}", self.system_output_text));
                        });
                        // input field
                        ui.vertical(|ui| {
                            ui.set_width(layout.input_station_width);
                            ui.set_height(layout.input_station_input_height);
                            ui.with_layout(egui::Layout::bottom_up(egui::Align::BOTTOM), |ui|{ // <----- add layout here
                                let text_input_edit = ui.text_edit_singleline(&mut self.user_input_text);
                                        ui.set_width(layout.input_station_width);
                                        if ui.input(|i| i.key_pressed(egui::Key::Slash)) {
                                           text_input_edit.request_focus(); 
                                        }
                                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                                self.process_input();
                                }});
                            });
                        });
                    });
    });
    }
}