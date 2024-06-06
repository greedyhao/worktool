use std::{
    process::Command,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use super::{convert_file_to_utf8, detect_encoding, preview_files_being_dropped};
use crate::{add_drop_file, component::Interface};

static HCI_TOOL_PAGE_KEY: &'static str = "HciKey";

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct HciToolSave {
    visable: bool,
    program: String,
}

pub struct HciToolPage {
    save: HciToolSave,
    doing: bool,
    channel: (Sender<()>, Receiver<()>),
    path: String,
    history: Option<String>,
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

                if let Ok(_) = self.channel.1.try_recv() {
                    self.doing = false;
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

impl HciToolPage {
    fn grid_contents(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.label("log2cfa 路径（会保存）");
        ui.text_edit_singleline(&mut self.save.program);
        ui.end_row();

        ui.label("转换文件路径");
        ui.text_edit_singleline(&mut self.path);
        ui.end_row();

        ui.add_enabled_ui(
            !self.doing && self.path.len() > 0 && self.save.program.len() > 0,
            |ui| {
                if ui.button("处理").clicked() {
                    self.doing = true;
                    let tx = self.channel.0.clone();
                    let program = self.save.program.clone();
                    let path = self.path.clone();
                    thread::spawn(move || {
                        if let Some(encode) = detect_encoding(&path) {
                            convert_file_to_utf8(&path, &encode).unwrap();
                        }
                        Command::new(program).arg(path).output().unwrap();
                        tx.send(()).unwrap();
                    });
                }
            },
        );
        ui.end_row();
    }
}
