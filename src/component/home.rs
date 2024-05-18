use super::UIPageFun;

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub struct HomePage {}

impl UIPageFun for HomePage {
    fn update(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Home");
        ui.label("在右侧选择对应的功能");
    }
}
