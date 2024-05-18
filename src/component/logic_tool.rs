use serde::Deserialize;
use std::{
    fs::File,
    io::{BufRead, BufReader, Lines, Write},
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use super::{preview_files_being_dropped, UIPageFun};

#[derive(Copy, Clone, PartialEq, Debug, Deserialize)]
enum SpiConvType {
    RAW,
    BluetrumVoiceDump,
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

// #[derive(PartialEq, Debug)]
// enum Protocal {
//     SPI(LogicSpiArgs),
//     IIS(LogicIISArgs),
// }

#[derive(PartialEq, Debug)]
enum Protocal {
    SPI,
    IIS,
}

#[repr(C)]
union ProtocalArgs {
    spi: LogicSpiArgs,
    iis: LogicIISArgs,
}

pub struct LogicToolPage {
    protocal: Protocal,
    path: String,
    doing: bool,
    channel: (Sender<bool>, Receiver<bool>),
    arg: ProtocalArgs,
}

impl LogicToolPage {
    pub fn new() -> Self {
        LogicToolPage {
            protocal: Protocal::SPI,
            path: String::new(),
            doing: false,
            channel: mpsc::channel(),
            arg: ProtocalArgs {
                spi: LogicSpiArgs {
                    conv_type: SpiConvType::RAW,
                },
            },
        }
    }
}

impl UIPageFun for LogicToolPage {
    fn update(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Logic Tool");

        ui.horizontal(|ui| {
            ui.label("协议类型");
            // ui.radio_value(
            //     &mut self.protocal,
            //     Protocal::SPI(LogicSpiArgs::default()),
            //     "SPI",
            // );
            // ui.radio_value(
            //     &mut self.protocal,
            //     Protocal::IIS(LogicIISArgs::default()),
            //     "IIS",
            // );
            ui.radio_value(&mut self.protocal, Protocal::SPI, "SPI");
            ui.radio_value(&mut self.protocal, Protocal::IIS, "IIS");
        });

        ui.horizontal(|ui| {
            ui.label("文件地址");
            ui.text_edit_singleline(&mut self.path);
        });

        if let Ok(doing) = self.channel.1.try_recv() {
            self.doing = doing;
        }

        match &mut self.protocal {
            Protocal::SPI => {
                ui.horizontal(|ui| {
                    ui.label("spi 格式");
                    unsafe {
                    ui.radio_value(&mut self.arg.spi.conv_type, SpiConvType::RAW, "RAW");
                    ui.radio_value(
                        &mut self.arg.spi.conv_type,
                        SpiConvType::BluetrumVoiceDump,
                        "蓝讯音频 DUMP 格式",
                    );
                }});
                ui.separator();
                ui.add_enabled_ui(!self.doing, |ui| {
                    if ui.button("处理").clicked() {
                        self.doing = true;
                        let tx = self.channel.0.clone();
                        let path = self.path.clone();
                        let arg;
                        unsafe {
                            arg = self.arg.spi.clone();
                        }
                        thread::spawn(move || {
                            logic_tool_proc_spi(&arg, &path);
                            tx.send(false).unwrap();
                        });
                    }
                });
            }
            Protocal::IIS => {
                ui.separator();
                ui.add_enabled_ui(!self.doing, |ui| {
                    if ui.button("处理").clicked() {
                        self.doing = true;
                        let tx = self.channel.0.clone();
                        let path = self.path.clone();
                        let arg;
                        unsafe {
                            arg = self.arg.iis.clone();
                        }
                        thread::spawn(move || {
                            logic_tool_proc_iis(&arg, &path);
                            tx.send(false).unwrap();
                        });
                    }
                });
            }
        }
        preview_files_being_dropped(ctx, &mut self.path);
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

const KINGST_ERROR_STR: &'static str =
    "The initial (idle) state of the CLK line does not match the settings";

const KINGST_IIS_FILE_FORMAT: &'static str = "Time [s],Channel,Value";
fn logic_tool_proc_iis(_args: &LogicIISArgs, path: &str) {
    let conv_file = path;

    if let Some(src) = logic_tool_preproc(&conv_file, KINGST_IIS_FILE_FORMAT) {
        println!("open {} success", &conv_file);
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

const KINGST_SPI_FILE_FORMAT: &'static str = "Time [s],Packet ID,MOSI,MISO";

fn logic_tool_proc_spi_raw(conv_file: &str) {
    if let Some(src) = logic_tool_preproc(conv_file, KINGST_SPI_FILE_FORMAT) {
        println!("open {} success", conv_file);
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

fn logic_tool_proc_spi(args: &LogicSpiArgs, path: &str) {
    match args.conv_type {
        SpiConvType::RAW => logic_tool_proc_spi_raw(path),
        SpiConvType::BluetrumVoiceDump => logic_tool_proc_spi_bluetrum(path),
    }
}