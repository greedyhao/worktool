use std::collections::HashMap;

use crate::component::{HardfaultToolPage, HciToolPage, Interface, LogicToolPage};
use once_cell::sync::Lazy;

const INTERFACE_TABLE: Lazy<std::vec::Vec<(&str, ActiveInterface)>> = Lazy::new(|| {
    vec![
        ("LogicTool", ActiveInterface::LogicTool),
        ("HardfaultTool", ActiveInterface::HardfaultTool),
        ("HciTool", ActiveInterface::Hcitool),
    ]
});

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq, Hash)]
enum ActiveInterface {
    Home,
    LogicTool,
    HardfaultTool,
    Hcitool,
}

pub struct WorkToolApp {
    interfaces: HashMap<ActiveInterface, Box<dyn Interface>>,
}

impl WorkToolApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);
        Self {
            interfaces: {
                let mut interfaces: HashMap<ActiveInterface, Box<dyn Interface>> = HashMap::new();
                interfaces.insert(ActiveInterface::LogicTool, Box::new(LogicToolPage::new(cc)));
                interfaces.insert(
                    ActiveInterface::HardfaultTool,
                    Box::new(HardfaultToolPage::new(cc)),
                );
                interfaces.insert(ActiveInterface::Hcitool, Box::new(HciToolPage::new(cc)));
                interfaces
            },
        }
    }
}

impl eframe::App for WorkToolApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let checkbox = INTERFACE_TABLE;

        for (_, interface) in checkbox.iter() {
            if let Some(interface) = self.interfaces.get_mut(&interface) {
                interface.save(storage);
            }
        }
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            ui.heading("子页面选择");
            ui.separator();

            let checkbox = INTERFACE_TABLE;

            for (label, interface) in checkbox.iter() {
                if let Some(interface) = self.interfaces.get_mut(&interface) {
                    ui.checkbox(&mut interface.get_mut_visable(), *label);
                    interface.update(ctx, frame);
                }
            }
            ui.separator();
            egui::widgets::global_dark_light_mode_buttons(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Home");
            ui.label("在右侧选择对应的功能");
            ui.label("需要处理的文件可以直接拖入对应窗口");
            ui.separator();

            ui.label("输入文件编码的说明");
            ui.label("utf8 不会转化，other 会自己猜测编码");
            ui.label("建议自己选择编码格式，猜测的编码可能会不对");
            ui.label("hci tool 的话，存在中文字符就用other，避免转完之后还有中文；log2cfa.exe 不支持中文字符");
            ui.label("转换完后，会有 .bak 文件作为备份");
            ui.separator();

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.hyperlink("https://github.com/greedyhao/worktool");
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();

    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    #[cfg(target_os = "windows")]
    fonts.font_data.insert(
        "my_font".to_owned(),
        egui::FontData::from_static(include_bytes!("c:/Windows/Fonts/msyh.ttc")),
    );

    // Put my font first (highest priority) for proportional text:
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "my_font".to_owned());

    // Put my font as last fallback for monospace:
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("my_font".to_owned());

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}
