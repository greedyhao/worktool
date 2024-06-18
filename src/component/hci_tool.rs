use std::{
    process::Command,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use super::{file_encoding_proc, file_encoding_select, preview_files_being_dropped, FileEncoding};
use crate::{add_drop_file, component::Interface};

static HCI_TOOL_PAGE_KEY: &str = "HciKey";

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct HciToolSave {
    visable: bool,
    program: String,
}

pub struct HciToolPage {
    save: HciToolSave,
    doing: bool,
    channel: (Sender<bool>, Receiver<bool>),
    path: String,
    history: Option<String>,
    file_encoding: FileEncoding,
}

add_drop_file!(HciToolPage);

impl eframe::App for HciToolPage {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, HCI_TOOL_PAGE_KEY, &self.save);
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.save.visable {
            egui::Window::new("HciTool").show(ctx, |ui| {
                ui.heading("HCI Tool");

                egui::Grid::new("hci")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| self.grid_contents(ctx, ui));

                if let Ok(status) = self.channel.1.try_recv() {
                    self.doing = false;
                    if !status {
                        self.file_encoding = FileEncoding::Other;
                    }
                }

                self.get_drop_file(ctx, ui);
            });
        }
    }
}

impl Interface for HciToolPage {
    fn new(cc: &eframe::CreationContext<'_>) -> Self
    where
        Self: Sized,
    {
        let mut page = HciToolPage {
            save: HciToolSave::default(),
            doing: false,
            channel: mpsc::channel(),
            path: String::new(),
            history: None,
            file_encoding: FileEncoding::UTF8,
        };

        if let Some(storage) = cc.storage {
            page.save = eframe::get_value(storage, HCI_TOOL_PAGE_KEY).unwrap_or_default();
        }
        page
    }
    fn get_mut_visable(&mut self) -> &mut bool {
        return &mut self.save.visable;
    }
}

fn hci_file_preproc(path: &str, tx: Sender<bool>, encode: &FileEncoding) {
    use regex::*;
    use std::fs;

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            tx.send(false).unwrap();
            return;
        }
    };

    // 定义正则表达式
    let re = Regex::new(r"\(\d{2}:\d{2}:\d{2}\.\d{3}\)").unwrap();
    // 替换匹配的字符串，前面添加回车符
    let modified_content = re.replace_all(&content, "");

    let re = Regex::new(r"\[\d{2}:\d{2}:\d{2}\.\d{3}\]").unwrap();

    let mut result = String::new();
    for line in modified_content.lines() {
        if line.len() == 0 {
            continue;
        }
        if re.is_match(line) {
            if ((line.contains("CMD ") || line.contains("EVT ") || line.contains("ACL "))
                && (line.contains(" => ") || line.contains(" <= ")))
                || (line.contains("MSG ") && (line.contains(" -> ") || line.contains(" <- ")))
            {
                result.push_str(re.replace_all(line, "\n$0").as_ref());
            } else {
                result.push_str(re.replace_all(line, "").as_ref());
            }
        } else {
            result.push_str(line);
        }
        result.push_str("\n");
    }

    if *encode == FileEncoding::UTF8 {
        fs::rename(path, &format!("{}.old", path)).unwrap();
    }
    let result: String = result.chars().filter(|c| *c < 128 as char).collect();
    fs::write(path, result).expect("Failed to write to the file");
}

impl HciToolPage {
    fn grid_contents(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.label("log2cfa 路径（会保存）");
        ui.text_edit_singleline(&mut self.save.program);
        ui.end_row();

        ui.label("转换文件路径");
        ui.text_edit_singleline(&mut self.path);
        ui.end_row();

        file_encoding_select(ui, &mut self.file_encoding);

        ui.add_enabled_ui(
            !self.doing && self.path.len() > 0 && self.save.program.len() > 0,
            |ui| {
                if ui.button("处理").clicked() {
                    self.doing = true;
                    let tx = self.channel.0.clone();
                    let program = self.save.program.clone();
                    let path = self.path.clone();
                    let encode = self.file_encoding.clone();
                    thread::spawn(move || {
                        file_encoding_proc(&path, &encode);
                        hci_file_preproc(&path, tx.clone(), &encode);
                        Command::new(program).arg(path).output().unwrap();
                        tx.send(true).unwrap();
                    });
                }
            },
        );
        ui.end_row();
    }
}
