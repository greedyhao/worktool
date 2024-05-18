mod hardfault_tool;
mod home;
mod logic_tool;

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
