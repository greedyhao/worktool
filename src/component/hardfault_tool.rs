use serde::Serialize;
use std::io::Read;
use std::{
    fs::File,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use super::{convert_file_to_utf8, detect_encoding};
use crate::component::preview_files_being_dropped;
use crate::component::Interface;

static HARDFAULT_TOOL_PAGE_KEY: &'static str = "HardfaultKey";

#[derive(Debug, Default, Serialize, Clone)]
pub struct CPURegs {
    regs: [String; 32],
    header: String,
}

static REG_NAME: [&'static str; 32] = [
    "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1", "a0", "a1", "a2", "a3", "a4",
    "a5", "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11", "t3", "t4",
    "t5", "t6",
];

impl CPURegs {
    fn display(&self) -> String {
        let mut ret = String::new();
        ret.push_str(&format!("{}\n", self.header));
        for (i, reg) in self.regs.iter().enumerate() {
            if i > 0 && (i % 4 == 0) {
                ret.push('\n');
            }
            ret.push_str(&format!("{}: {}, ", REG_NAME[i], reg));
        }
        ret
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
struct HardfaultToolSave {
    visable: bool,
}

pub struct HardfaultToolPage {
    save: HardfaultToolSave,
    path: String,
    history: Option<String>,
    channel: (Sender<Vec<CPURegs>>, Receiver<Vec<CPURegs>>),
    doing: bool,
    regs: Vec<CPURegs>,
    selected: usize,
}

impl eframe::App for HardfaultToolPage {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, HARDFAULT_TOOL_PAGE_KEY, &self.save);
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.save.visable {
            egui::Window::new("HardfaultTool").show(ctx, |ui| {
                egui::Grid::new("hardfault")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| self.grid_contents(ctx, ui));

                // for reg in &self.regs {
                //     ui.label(reg.display());
                // }
                if self.regs.len() > 0 {
                    ui.label(self.regs[self.selected].display());
                }

                if let Ok(regs) = self.channel.1.try_recv() {
                    self.doing = false;
                    self.regs = regs;
                    self.selected = 0;
                }

                ctx.input(|i| {
                    if let Some(point) = i.pointer.latest_pos() {
                        if let Some(path) = &self.history {
                            if ui.min_rect().contains(point) {
                                self.path = path.to_string();
                            }
                            self.history = None;
                        }
                    }
                });

                if let Some(path) = preview_files_being_dropped(ctx) {
                    self.history = Some(path);
                    ctx.request_repaint();
                }
            });
        }
    }
}

impl Interface for HardfaultToolPage {
    fn new(cc: &eframe::CreationContext<'_>) -> Self
    where
        Self: Sized,
    {
        let mut page = HardfaultToolPage {
            save: HardfaultToolSave::default(),
            path: String::new(),
            history: None,
            channel: mpsc::channel(),
            doing: false,
            regs: Vec::new(),
            selected: 0,
        };

        if let Some(storage) = cc.storage {
            page.save = eframe::get_value(storage, HARDFAULT_TOOL_PAGE_KEY).unwrap_or_default();
        }
        page
    }
    fn get_mut_visable(&mut self) -> &mut bool {
        return &mut self.save.visable;
    }
}

impl HardfaultToolPage {
    fn grid_contents(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.label("文件地址");
        ui.text_edit_singleline(&mut self.path);
        ui.end_row();

        ui.label("选择需要显示的寄存器组");
        ui.end_row();
        ui.add_enabled_ui(self.regs.len() > 0, |ui| {
            egui::ComboBox::from_label("寄存器组")
                .selected_text(format!("{}", self.selected))
                .show_ui(ui, |ui| {
                    let len = self.regs.len();
                    for i in 0..len {
                        ui.selectable_value(&mut self.selected, i, format!("{}", i));
                    }
                });
        });
        ui.end_row();

        ui.add_enabled_ui(!self.doing && self.path.len() > 0, |ui| {
            if ui.button("处理").clicked() {
                self.doing = true;
                let tx = self.channel.0.clone();
                let path = self.path.clone();
                thread::spawn(move || {
                    let ret = hardfault_tool(path);
                    tx.send(ret).unwrap();
                });
            }
        });
        ui.end_row();
    }
}

fn hardfault_tool(path: String) -> Vec<CPURegs> {
    let start_flag1 = "ERR:";
    let start_flag2 = "EPC:";
    let start_flag3 = "WDT_RST:";

    let empty_str = "0xXXXXXXXX";
    let mut regs = CPURegs::default();
    let mut reg_vec = Vec::new();

    if let Some(encode) = detect_encoding(&path) {
        convert_file_to_utf8(&path, &encode).unwrap();
    }

    if let Ok(mut file) = File::open(&path) {
        println!("open {} success", &path);

        let mut index = 0;
        let mut state = 0; // 1: epc, 2: wdt

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        let lines = String::from_utf8(buf.to_vec()).unwrap();

        for line in lines.split(|c| c == '\r' || c == '\n') {
            // println!("line: {}, state:{}", line, state);
            match state {
                1 => {
                    for l in line.split(' ') {
                        if l.len() == 0 {
                            continue;
                        }
                        if let Ok(reg) = u32::from_str_radix(l, 16) {
                            regs.regs[index] = format!("{:#010X}", reg);
                        } else {
                            state = 3;
                        }

                        index += 1;
                    }
                    if index >= 32 {
                        state = 3;
                        reg_vec.push(regs.clone());
                    }
                }
                2 => {
                    for l in line.split(' ') {
                        match index {
                            0 => {
                                regs.regs[index] = empty_str.to_string();
                                index += 1;
                            }
                            2 => {
                                while index < 4 {
                                    regs.regs[index] = empty_str.to_string();
                                    index += 1;
                                }
                            }
                            18 => {
                                while index < 28 {
                                    regs.regs[index] = empty_str.to_string();
                                    index += 1;
                                }
                            }
                            _ => {}
                        }
                        if l.len() == 0 {
                            continue;
                        }

                        if let Ok(reg) = u32::from_str_radix(l, 16) {
                            regs.regs[index] = format!("{:#010X}", reg);
                        } else {
                            state = 3;
                        }

                        index += 1;
                    }
                    if index >= 19 {
                        state = 3;
                        reg_vec.push(regs.clone());
                    }
                }
                _ => {}
            }

            if line.contains(start_flag1) && line.contains(start_flag2) {
                regs.header = line.to_string();
                state = 1;
                index = 0;
                // println!("EPC");
            }
            if line.contains(start_flag3) {
                regs.header = line.to_string();
                state = 2;
                index = 0;
                // println!("WDT");
            }
        }
    }

    reg_vec
}
