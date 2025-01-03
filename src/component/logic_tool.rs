use serde::Deserialize;
use std::{
    fs::File,
    io::{BufRead, BufReader, Lines, Write},
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use super::{preview_files_being_dropped, show_page_header};
use crate::{add_drop_file, component::Interface};

static LOGIC_TOOL_PAGE_KEY: &str = "LogicKey";

#[derive(Copy, Clone, PartialEq, Debug, Deserialize)]
enum SpiConvType {
    RAW,
    BluetrumVoiceDump,
    TXT,
}

#[derive(Copy, Clone, PartialEq, Debug)]
struct LogicSpiArgs {
    conv_type: SpiConvType,
}

impl Default for LogicSpiArgs {
    fn default() -> Self {
        LogicSpiArgs {
            conv_type: SpiConvType::RAW,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct LogicIISArgs {}

impl Default for LogicIISArgs {
    fn default() -> Self {
        LogicIISArgs {}
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct LogicUARTArgs {}

impl Default for LogicUARTArgs {
    fn default() -> Self {
        LogicUARTArgs {}
    }
}

#[derive(PartialEq, Debug, Clone, Default)]
enum Protocal {
    #[default]
    SPI,
    IIS,
    UART,
}

#[derive(Debug)]
struct ProtocalArgs {
    spi: LogicSpiArgs,
    iis: LogicIISArgs,
    uart: LogicUARTArgs,
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
struct LogicToolPageSave {
    visable: bool,
}

#[derive(Debug)]
pub struct LogicToolPage {
    save: LogicToolPageSave,
    protocal: Protocal,
    path: String,
    history: Option<String>,
    doing: bool,
    channel: (Sender<bool>, Receiver<bool>),
    arg: ProtocalArgs,
}

add_drop_file!(LogicToolPage);

impl eframe::App for LogicToolPage {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, LOGIC_TOOL_PAGE_KEY, &self.save);
    }
    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {}
}

impl Interface for LogicToolPage {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut page = LogicToolPage {
            save: LogicToolPageSave { visable: false },
            protocal: Protocal::SPI,
            path: String::new(),
            history: None,
            doing: false,
            channel: mpsc::channel(),
            arg: ProtocalArgs {
                spi: LogicSpiArgs {
                    conv_type: SpiConvType::RAW,
                },
                iis: LogicIISArgs::default(),
                uart: LogicUARTArgs::default(),
            },
        };

        if let Some(storage) = cc.storage {
            page.save = eframe::get_value(storage, LOGIC_TOOL_PAGE_KEY).unwrap_or_default();
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

        ui.heading("Logic Tool");

        egui::Grid::new("hardfault")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| self.grid_contents(ctx, ui));

        if let Ok(doing) = self.channel.1.try_recv() {
            self.doing = doing;
        }

        self.get_drop_file(ctx, ui);
    }
}

impl LogicToolPage {
    fn grid_contents(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.label("协议类型");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.protocal, Protocal::SPI, "SPI");
            ui.radio_value(&mut self.protocal, Protocal::IIS, "IIS");
            ui.radio_value(&mut self.protocal, Protocal::UART, "UART");
        });
        ui.end_row();

        ui.label("文件地址");
        ui.text_edit_singleline(&mut self.path);
        ui.end_row();

        match &mut self.protocal {
            Protocal::SPI => {
                ui.label("spi 格式");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.arg.spi.conv_type, SpiConvType::RAW, "RAW");
                    ui.radio_value(
                        &mut self.arg.spi.conv_type,
                        SpiConvType::BluetrumVoiceDump,
                        "蓝讯音频 DUMP 格式",
                    );
                    ui.radio_value(&mut self.arg.spi.conv_type, SpiConvType::TXT, "TXT");
                });
                ui.end_row();
                ui.add_enabled_ui(!self.doing && self.path.len() > 0, |ui| {
                    if ui.button("处理").clicked() {
                        self.doing = true;
                        let tx = self.channel.0.clone();
                        let path = self.path.clone();
                        let arg;
                        arg = self.arg.spi.clone();
                        thread::spawn(move || {
                            logic_tool_proc_spi(&arg, &path);
                            tx.send(false).unwrap();
                        });
                    }
                });
                ui.end_row();
            }
            Protocal::IIS => {
                ui.add_enabled_ui(!self.doing && self.path.len() > 0, |ui| {
                    if ui.button("处理").clicked() {
                        self.doing = true;
                        let tx = self.channel.0.clone();
                        let path = self.path.clone();
                        let arg = self.arg.iis.clone();
                        thread::spawn(move || {
                            logic_tool_proc_iis(&arg, &path);
                            tx.send(false).unwrap();
                        });
                    }
                });
                ui.end_row();
            }
            Protocal::UART => {
                ui.add_enabled_ui(!self.doing && self.path.len() > 0, |ui| {
                    if ui.button("处理").clicked() {
                        self.doing = true;
                        let tx = self.channel.0.clone();
                        let path = self.path.clone();
                        let arg = self.arg.uart.clone();
                        thread::spawn(move || {
                            logic_tool_proc_uart(&arg, &path);
                            tx.send(false).unwrap();
                        });
                    }
                });
                ui.end_row();
            }
        }
    }
}

fn logic_tool_preproc(in_file: &str, header: &str) -> Option<Lines<BufReader<File>>> {
    if let Ok(file) = File::open(&in_file) {
        let mut lines = BufReader::new(file).lines();
        let first = lines.next();
        if let Some(first) = first {
            if first.unwrap().contains(header) {
                return Some(lines);
            }
        }
    }
    return None;
}

const KINGST_ERROR_STR: &str =
    "The initial (idle) state of the CLK line does not match the settings";

const KINGST_IIS_FILE_FORMAT: &str = "Time [s],Channel,Value";
fn logic_tool_proc_iis(_args: &LogicIISArgs, path: &str) {
    let conv_file = path;

    if let Some(src) = logic_tool_preproc(&conv_file, KINGST_IIS_FILE_FORMAT) {
        // println!("open {} success", &conv_file);
        let out_path = format!("{}.out", &conv_file);
        let mut out_file = File::create(out_path).unwrap();

        for line in src {
            if let Ok(line) = line {
                // 跳过错误数据
                if line.contains(KINGST_ERROR_STR) {
                    continue;
                }

                let data: String = line.split(',').filter(|w| w.contains("0x")).collect();
                let data = data.trim_start_matches("0x");
                let data = u16::from_str_radix(data, 16).unwrap();
                out_file.write(&data.to_le_bytes()).unwrap();
            }
        }
    }
}

const KINGST_SPI_FILE_FORMAT: &str = "Time [s],Packet ID,MOSI,MISO";

fn logic_tool_proc_spi_raw(conv_file: &str) {
    if let Some(src) = logic_tool_preproc(conv_file, KINGST_SPI_FILE_FORMAT) {
        // println!("open {} success", conv_file);
        let out_path = format!("{}.out", conv_file);
        let mut out_file = File::create(out_path).unwrap();

        for line in src {
            if let Ok(line) = line {
                // 跳过错误数据
                if line.contains(KINGST_ERROR_STR) {
                    continue;
                }

                let data: String = line.split(',').filter(|w| w.contains("0x")).collect();
                let data = data.trim_start_matches("0x");
                let data = u8::from_str_radix(data, 16).unwrap();
                out_file.write(&[data]).unwrap();
            }
        }
    }
}

#[derive(Debug, Default)]
struct BluetrumVoiceDump {
    version: [u8; 4],
    frame_type: u8,
    len: u16,
    frame_num: u8,
}

enum BluetrumVoiceDumpState {
    Header,
    Body,
}

fn logic_tool_proc_spi_bluetrum(conv_file: &str) {
    if let Some(src) = logic_tool_preproc(conv_file, KINGST_SPI_FILE_FORMAT) {
        let out1_path = format!("{}.out1", conv_file);
        let out2_path = format!("{}.out2", conv_file);
        let out3_path = format!("{}.out3", conv_file);

        let mut out1_file = File::create(out1_path).unwrap();
        let mut out2_file = File::create(out2_path).unwrap();
        let mut out3_file = File::create(out3_path).unwrap();

        let mut cnt = 0;
        let mut header = BluetrumVoiceDump::default();
        let mut header_cache = Vec::new();
        let mut state = BluetrumVoiceDumpState::Header;
        for line in src {
            if let Ok(line) = line {
                // 跳过错误数据
                if line.contains(KINGST_ERROR_STR) {
                    continue;
                }

                let data: String = line.split(',').filter(|w| w.contains("0x")).collect();
                let data = data.trim_start_matches("0x");
                let data = u8::from_str_radix(data, 16).unwrap();
                // out_file.write(&[data]).unwrap();
                // println!("line:{} - {:x}", line, data);

                match state {
                    BluetrumVoiceDumpState::Header => {
                        header_cache.push(data);
                        if header_cache.len() == 8 {
                            state = BluetrumVoiceDumpState::Body;
                            header = BluetrumVoiceDump {
                                version: header_cache[0..4].try_into().unwrap(),
                                frame_type: header_cache[4],
                                len: (header_cache[5] as u16) << 8 | (header_cache[6] as u16),
                                // len: u16::from_be_bytes(header_cache[5..6].try_into().unwrap()),
                                frame_num: header_cache[7],
                            };
                            println!("{:?} {}", header.version, header.frame_num);
                        }
                    }
                    BluetrumVoiceDumpState::Body => {
                        match header.frame_type {
                            0 => {
                                out1_file.write(&[data]).unwrap();
                            }
                            2 => {
                                out2_file.write(&[data]).unwrap();
                            }
                            4 => {
                                out3_file.write(&[data]).unwrap();
                            }
                            _ => {}
                        };

                        cnt += 1;
                        if cnt == header.len {
                            state = BluetrumVoiceDumpState::Header;
                            header_cache.clear();
                            cnt = 0;
                        }
                    }
                }
            }
        }
    }
}

fn logic_tool_proc_spi_txt(conv_file: &str) {
    if let Some(src) = logic_tool_preproc(conv_file, KINGST_SPI_FILE_FORMAT) {
        // println!("open {} success", conv_file);
        let out_path = format!("{}.txt", conv_file);
        let mut out_file = File::create(out_path).unwrap();

        let mut cnt = 0;
        for line in src {
            if let Ok(line) = line {
                // 跳过错误数据
                if line.contains(KINGST_ERROR_STR) {
                    continue;
                }

                let data: String = line.split(',').filter(|w| w.contains("0x")).collect();
                let data = data.trim_start_matches("0x");
                let data = u8::from_str_radix(data, 16).unwrap();
                // out_file.write(&[data]).unwrap();
                if cnt > 0 && (cnt % 16 == 0) {
                    write!(out_file, "\n").unwrap();
                }
                write!(out_file, "{:02x} ", data).unwrap();
                cnt += 1;
            }
        }
    }
}

fn logic_tool_proc_spi(args: &LogicSpiArgs, path: &str) {
    match args.conv_type {
        SpiConvType::RAW => logic_tool_proc_spi_raw(path),
        SpiConvType::BluetrumVoiceDump => logic_tool_proc_spi_bluetrum(path),
        SpiConvType::TXT => logic_tool_proc_spi_txt(path),
    }
}

const KINGST_UART_FILE_FORMAT: &str = "Time [s],Value,Parity Error,Framing Error";

fn logic_tool_proc_uart_txt(conv_file: &str) {
    if let Some(src) = logic_tool_preproc(conv_file, KINGST_UART_FILE_FORMAT) {
        // println!("open {} success", conv_file);
        let out_path = format!("{}.txt", conv_file);
        let mut out_file = File::create(out_path).unwrap();

        let mut cnt = 0;
        for line in src {
            if let Ok(line) = line {
                // 跳过错误数据
                if line.contains(KINGST_ERROR_STR) {
                    continue;
                }

                let data: String = line.split(',').filter(|w| w.contains("0x")).collect();
                let data = data.trim_start_matches("0x");
                let data = u8::from_str_radix(data, 16).unwrap();
                // out_file.write(&[data]).unwrap();
                if cnt > 0 && (cnt % 16 == 0) {
                    write!(out_file, "\n").unwrap();
                }
                write!(out_file, "{:02x} ", data).unwrap();
                cnt += 1;
            }
        }
    }
}

fn logic_tool_proc_uart(_args: &LogicUARTArgs, path: &str) {
    logic_tool_proc_uart_txt(path);
}
