use std::{
    process::Command,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use super::{
    convert_file_to_utf8, detect_encoding, preview_files_being_dropped, UIPageFun, UIPageSave,
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct HciToolSave {
    program: String,
}
impl Default for HciToolSave {
    fn default() -> Self {
        HciToolSave {
            program: String::new(),
        }
    }
}

pub struct HciToolPage {
    doing: bool,
    channel: (Sender<()>, Receiver<()>),
    path: String,
}

impl HciToolPage {
    pub fn new() -> Self {
        HciToolPage {
            doing: false,
            channel: mpsc::channel(),
            path: String::new(),
        }
    }
    fn grid_contents(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, save: &mut HciToolSave) {
        ui.label("log2cfa 路径（会保存）");
        ui.text_edit_singleline(&mut save.program);
        ui.end_row();

        ui.label("转换文件路径");
        ui.text_edit_singleline(&mut self.path);
        ui.end_row();

        ui.add_enabled_ui(
            !self.doing && self.path.len() > 0 && save.program.len() > 0,
            |ui| {
                if ui.button("处理").clicked() {
                    self.doing = true;
                    let tx = self.channel.0.clone();
                    let program = save.program.clone();
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

impl UIPageFun for HciToolPage {
    fn update(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, save: &mut UIPageSave) {
        ui.heading("HCI Tool");

        egui::Grid::new("hci")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| self.grid_contents(ctx, ui, &mut save.hci_tool));

        if let Ok(_) = self.channel.1.try_recv() {
            self.doing = false;
        }
        preview_files_being_dropped(ctx, &mut self.path);
    }
}
