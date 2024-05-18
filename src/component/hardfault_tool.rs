use serde::Serialize;
use std::{
    borrow::Cow,
    fs::File,
    io::{BufRead, BufReader},
};

use super::UIPageFun;

#[derive(Debug, Default, Serialize, Clone)]
pub struct CPURegs {
    regs: [String; 32],
    header: String,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub struct HardfaultToolPage {}

impl UIPageFun for HardfaultToolPage {
    fn update(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Hardfault Tool");
    }
}

fn _hardfault_tool<'a>(path: Cow<'a, str>) -> Vec<CPURegs> {
    let start_flag1 = "ERR:";
    let start_flag2 = "EPC:";
    let start_flag3 = "WDT_RST:";

    let empty_str = "0xXXXXXXXX";
    let mut regs = CPURegs::default();
    let mut reg_vec = Vec::new();
    let path = path.to_string();
    if let Ok(file) = File::open(&path) {
        println!("open {} success", &path);

        let mut index = 0;
        let mut state = 0; // 1: epc, 2: wdt
        let lines = BufReader::new(file).lines();
        for line in lines {
            if let Ok(line) = line {
                // println!("line: {}, state:{}", line, state);
                match state {
                    1 => {
                        for l in line.split(' ') {
                            if l.len() == 0 {
                                continue;
                            }

                            regs.regs[index] =
                                format!("{:#010X}", u32::from_str_radix(l, 16).unwrap());
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

                            regs.regs[index] =
                                format!("{:#010X}", u32::from_str_radix(l, 16).unwrap());
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
    }

    reg_vec
}
