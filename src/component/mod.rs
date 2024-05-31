mod hardfault_tool;
mod hci_tool;
mod home;
mod logic_tool;

use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

pub use self::hci_tool::*;
pub use hardfault_tool::HardfaultToolPage;
pub use home::HomePage;
pub use logic_tool::LogicToolPage;

pub enum UIPage {
    Home(HomePage),
    LogicTool(LogicToolPage),
    HardfaultTool(HardfaultToolPage),
    HciTool(HciToolPage),
}

impl UIPage {
    pub fn update(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, save: &mut UIPageSave) {
        match self {
            Self::Home(page) => page.update(ctx, ui, save),
            Self::LogicTool(page) => page.update(ctx, ui, save),
            Self::HardfaultTool(page) => page.update(ctx, ui, save),
            Self::HciTool(page) => page.update(ctx, ui, save),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct UIPageSave {
    hci_tool: HciToolSave,
}

impl Default for UIPageSave {
    fn default() -> Self {
        UIPageSave {
            hci_tool: HciToolSave::default(),
        }
    }
}

pub trait UIPageFun {
    fn update(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, save: &mut UIPageSave);
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

pub fn detect_encoding(path: &str) -> Option<String> {
    if let Ok(result) = charset_normalizer_rs::from_path(&PathBuf::from(path), None) {
        if let Some(best) = result.get_best() {
            return Some(best.encoding().to_uppercase().to_string());
        }
    }
    None
}

pub fn convert_file_to_utf8(path: &str, encoding_name: &str) -> std::io::Result<()> {
    use std::io::Read;

    let mut file = File::open(path)?;
    let output_path = format!("{}.tmp", path);

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    // println!("{}", buf.len());
    let encoding =
        encoding_rs::Encoding::for_label(encoding_name.as_bytes()).unwrap_or(encoding_rs::UTF_8);

    // 将字节向量解码为UTF-8
    let (decoded_str, _, had_errors) = encoding.decode(&buf);

    if had_errors {
        eprintln!("Warning: Some characters could not be decoded correctly.");
    }

    // 打开输出文件
    let mut output_file = File::create(&output_path)?;

    // 将解码后的字符串写入输出文件
    output_file.write_all(decoded_str.as_bytes())?;

    fs::rename(path, &format!("{}.bak", path))?;
    fs::rename(&output_path, path)?;
    Ok(())
}
