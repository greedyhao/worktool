use std::collections::HashMap;
use std::sync::Arc;

use crate::component::{AnalyzeToolPage, HardfaultToolPage, HciToolPage, Interface, LogicToolPage};

use egui::vec2;
use egui::{ScrollArea, Ui};

use num_enum::{IntoPrimitive, TryFromPrimitive};

include!(concat!(env!("OUT_DIR"), "/info.rs"));

#[derive(
    serde::Deserialize, serde::Serialize, PartialEq, Eq, Hash, TryFromPrimitive, IntoPrimitive,
)]
#[repr(u32)]
enum ActiveInterface {
    Home,
    LogicTool,
    HardfaultTool,
    HciTool,
    AnalyzeTool,
}

impl ActiveInterface {
    fn as_str(&self) -> &'static str {
        match self {
            ActiveInterface::Home => "Home",
            ActiveInterface::HardfaultTool => "Hardfault Tool",
            ActiveInterface::HciTool => "Hci Tool",
            ActiveInterface::LogicTool => "Logic Tool",
            ActiveInterface::AnalyzeTool => "Analyze Tool",
        }
    }
}

pub struct WorkToolApp {
    interfaces: HashMap<ActiveInterface, Box<dyn Interface>>,
    current_page: ActiveInterface,
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
                interfaces.insert(ActiveInterface::HciTool, Box::new(HciToolPage::new(cc)));
                interfaces.insert(
                    ActiveInterface::AnalyzeTool,
                    Box::new(AnalyzeToolPage::new(cc)),
                );
                interfaces
            },
            current_page: ActiveInterface::Home,
        }
    }

    fn show_main_page(&mut self, ui: &mut Ui) {
        // 上半部分：文字描述
        ui.heading("Home");
        ui.label(format!("编译时间：{}", COMPILE_TIME));
        ui.label(format!("git 信息：{} ({})", &GIT_HASH[0..8], GIT_TIMESTAMP));
        ui.label("");

        ui.label("在下方选择对应的功能");
        ui.label("需要处理的文件可以直接拖入对应窗口\n");

        ui.label("输入文件编码的说明：");
        ui.label("utf8 不会转化，other 会自己猜测编码");
        ui.label("建议自己选择编码格式，猜测的编码可能会不对");
        ui.label("hci tool 的话，存在中文字符就用other，避免转完之后还有中文；log2cfa.exe 不支持中文字符");
        ui.label("转换完后，会有 .bak 文件作为备份");
        ui.separator();

        // 下半部分：应用宫格排列
        ScrollArea::vertical().show(ui, |ui| {
            let line_size = 3;
            let total = self.interfaces.len();
            let line = (total + line_size - 1) / line_size;
            let mut cnt = 0;

            for l in 0..line {
                ui.columns(3, |columns| {
                    for (i, column) in columns.iter_mut().enumerate() {
                        cnt += 1;
                        if cnt > total {
                            break;
                        }

                        let page =
                            ActiveInterface::try_from((i + l * line_size + 1) as u32).unwrap();
                        let app_name = format!("{}", page.as_str());

                        if column
                            .add_sized(vec2(100.0, 100.0), egui::Button::new(&app_name))
                            .clicked()
                        {
                            self.current_page = page;
                        }
                    }
                });
            }
        });

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.hyperlink("https://github.com/greedyhao/worktool");
            egui::warn_if_debug_build(ui);
        });
    }
}

impl eframe::App for WorkToolApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        for (_, interface) in self.interfaces.iter_mut() {
            interface.save(storage);
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.current_page == ActiveInterface::Home {
                self.show_main_page(ui);
            } else {
                if let Some(interface) = self.interfaces.get_mut(&self.current_page) {
                    interface.new_update(
                        ui,
                        ctx,
                        Box::new(|| self.current_page = ActiveInterface::Home),
                    );
                }
            }
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
        Arc::new(egui::FontData::from_static(include_bytes!(
            "c:/Windows/Fonts/msyh.ttc"
        ))),
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
