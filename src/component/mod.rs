mod hardfault_tool;
mod home;
mod logic_tool;

use std::{fs::{self, File}, io::Write};

pub use hardfault_tool::HardfaultToolPage;
pub use home::HomePage;
pub use logic_tool::LogicToolPage;

// #[derive(serde::Deserialize, serde::Serialize)]
pub enum UIPage {
    Home(HomePage),
    LogicTool(LogicToolPage),
    HardfaultTool(HardfaultToolPage),
}

impl UIPage {
    pub fn update(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        match self {
            Self::Home(page) => page.update(ctx, ui),
            Self::LogicTool(page) => page.update(ctx, ui),
            Self::HardfaultTool(page) => page.update(ctx, ui),
        }
    }
}

pub trait UIPageFun {
    fn update(&mut self, ctx: &egui::Context, ui: &mut egui::Ui);
}

/// Preview hovering files:
pub fn preview_files_being_dropped(ctx: &egui::Context, drop_file: &mut String) {
    use egui::*;
    use std::fmt::Write as _;

    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let path = ctx.input(|i| {
            let mut res = String::new();
            for file in &i.raw.hovered_files {
                if let Some(path) = &file.path {
                    write!(res, "{}", path.display()).ok();
                } else if !file.mime.is_empty() {
                    write!(res, "{}", file.mime).ok();
                } else {
                    res += "???";
                }
            }
            res
        });
        let text = format!("Dropping files:\n\n{}", path);
        *drop_file = path;

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}

// const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];
const UTF16BE_BOM: [u8; 2] = [0xFE, 0xFF];
const UTF16LE_BOM: [u8; 2] = [0xFF, 0xFE];
const UTF32BE_BOM: [u8; 4] = [0x00, 0x00, 0xFE, 0xFF];
const UTF32LE_BOM: [u8; 4] = [0xFF, 0xFE, 0x00, 0x00];

#[derive(PartialEq)]
enum FileEncode {
    UTF8,
    UTF16BE,
    UTF16LE,
    UTF32BE,
    UTF32LE,
}

fn is_subarry(arr1: &[u8], arr2: &[u8]) -> bool {
    for (i, v) in arr1.iter().enumerate() {
        if &arr2[i] != v {
            return false;
        }
    }
    true
}

fn check_file_encoding(file: &str) -> Option<FileEncode> {
    use std::io::Read;
    if let Ok(mut file) = std::fs::File::open(&file) {
        let mut buffer = [0; 10];
        file.read(&mut buffer[..]).unwrap();
        println!("{:?}", buffer);

        let code = if is_subarry(&UTF16BE_BOM, &buffer) {
            FileEncode::UTF16BE
        } else if is_subarry(&UTF16LE_BOM, &buffer) {
            FileEncode::UTF16LE
        } else if is_subarry(&UTF32BE_BOM, &buffer) {
            FileEncode::UTF32BE
        } else if is_subarry(&UTF32LE_BOM, &buffer) {
            FileEncode::UTF32LE
        } else {
            FileEncode::UTF8
        };
        return Some(code);
    }
    None
}

pub fn convert_file_to_utf8(path: &str) {
    use std::io::Read;

    if let Some(code) = check_file_encoding(path) {
        if code == FileEncode::UTF8 {
            return;
        }
    }

    if let Ok(mut file) = File::open(path) {
        let new_path = format!("{}.tmp", path);
        let mut new = File::create(&new_path).unwrap();

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        // println!("{}", buf.len());
        let (out, _, _) = encoding_rs::UTF_8.decode(&buf);
        new.write(out.as_bytes()).unwrap();

        fs::remove_file(path).unwrap();
        fs::rename(&new_path, path).unwrap();
    }
}
