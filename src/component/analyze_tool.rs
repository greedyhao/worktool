use crate::add_drop_file;
use crate::component::preview_files_being_dropped;
use crate::component::show_page_header;
use crate::component::Interface;

use std::collections::HashSet;
use std::error::Error;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::sync::mpsc;
use std::{
    fs::File,
    sync::mpsc::{Receiver, Sender},
    thread,
};

static ANALYZE_TOOL_PAGE_KEY: &str = "AnalyzeKey";

#[derive(Default, serde::Deserialize, serde::Serialize)]
struct ToolSave {
    types: String,
}

pub struct AnalyzeToolPage {
    save: ToolSave,
    path: String,
    history: Option<String>,
    channel: (Sender<bool>, Receiver<bool>),
    doing: bool,
}

add_drop_file!(AnalyzeToolPage);

impl eframe::App for AnalyzeToolPage {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, ANALYZE_TOOL_PAGE_KEY, &self.save);
    }
    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {}
}

impl Interface for AnalyzeToolPage {
    fn new(cc: &eframe::CreationContext<'_>) -> Self
    where
        Self: Sized,
    {
        let mut page = AnalyzeToolPage {
            save: ToolSave::default(),
            path: String::new(),
            history: None,
            doing: false,
            channel: mpsc::channel(),
        };

        if let Some(storage) = cc.storage {
            page.save = eframe::get_value(storage, ANALYZE_TOOL_PAGE_KEY).unwrap_or_default();
        }
        page
    }
    fn new_update<'a>(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        close: Box<dyn FnMut() + 'a>,
    ) {
        show_page_header(ui, close);

        egui::Grid::new("hci")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| self.grid_contents(ctx, ui));

        if let Ok(_status) = self.channel.1.try_recv() {
            self.doing = false;
        }
        self.get_drop_file(ctx, ui);
    }
}

impl AnalyzeToolPage {
    fn grid_contents(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.label("文件地址");
        ui.text_edit_singleline(&mut self.path);
        ui.end_row();

        ui.label("需要转换的类型");
        ui.text_edit_singleline(&mut self.save.types);
        ui.end_row();

        ui.add_enabled_ui(!self.doing && self.path.len() > 0, |ui| {
            if ui.button("处理").clicked() {
                self.doing = true;
                let tx = self.channel.0.clone();
                let types = self.save.types.clone();
                let input_path = self.path.clone();
                let output_path = format!("{}.out.txt", input_path);

                thread::spawn(move || {
                    analyze_main(&types, &input_path, &output_path);
                    tx.send(true).unwrap();
                });
            }
        });
    }
}

#[derive(Debug)]
struct LogMessage {
    timestamp: f64,
    content: String,
}

fn process_logic_data(reader: impl BufRead) -> Result<Vec<LogMessage>, Box<dyn Error>> {
    let mut messages = Vec::new();
    let mut current_message = String::with_capacity(100); // 预分配内存
    let mut start_time: Option<f64> = None;

    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(reader);

    // 获取并缓存列索引
    let headers = csv_reader.headers()?;
    let time_idx = headers
        .iter()
        .position(|h| h == "Time [s]")
        .ok_or("Missing 'Time [s]' column")?;
    let mosi_idx = headers
        .iter()
        .position(|h| h == "MOSI")
        .ok_or("Missing 'MOSI' column")?;

    // 使用 into_records 避免克隆
    for result in csv_reader.into_records() {
        let record = result?;

        let time: f64 = record.get(time_idx).ok_or("Missing time field")?.parse()?;

        let char = record.get(mosi_idx).ok_or("Missing MOSI field")?;

        if start_time.is_none() {
            start_time = Some(time);
        }

        match char {
            "NUL" => current_message.push(' '),
            "LF " => {
                if !current_message.is_empty() {
                    let message = current_message.trim().to_string();
                    if message.contains(':') {
                        if let Some(timestamp) = start_time {
                            messages.push(LogMessage {
                                timestamp,
                                content: message,
                            });
                        }
                        start_time = None;
                    }
                }
                current_message.clear();
            }
            c => current_message.push_str(c),
        }
    }

    // let len = messages.len();

    Ok(messages)
}

#[inline]
fn validate_message_type(message: &str, map: &HashSet<String>) -> bool {
    if let Some(msg_type) = message.split(':').next() {
        map.contains(msg_type)
    } else {
        false
    }
}

fn write_output(messages: &[LogMessage], output_file: &str) -> io::Result<()> {
    let file = File::create(output_file)?;
    let mut writer = BufWriter::new(file);

    for message in messages {
        writeln!(writer, "[{:.6}]{}", message.timestamp, message.content)?;
    }

    writer.flush()?;
    Ok(())
}

fn analyze_main(types: &str, input_file: &str, output_file: &str) {
    let valid_types: HashSet<String> = types.split(',').map(|s| s.trim().to_string()).collect();

    let file = File::open(input_file).unwrap();
    let reader = BufReader::with_capacity(128 * 1024, file);

    let messages = process_logic_data(reader).unwrap();

    let valid_messages: Vec<_> = messages
        .into_iter()
        .filter(|msg| validate_message_type(&msg.content, &valid_types))
        .collect();
    write_output(&valid_messages, output_file).unwrap();
}
