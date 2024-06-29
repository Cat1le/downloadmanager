use eframe::egui::{
    pos2, vec2, Color32, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2, Widget,
};

pub struct ManyProgressBar<'ranges> {
    size: Vec2,
    ranges: &'ranges [(f32, f32, bool)],
}

impl ManyProgressBar<'_> {
    pub fn new(size: Vec2, ranges: &[(f32, f32, bool)]) -> ManyProgressBar<'_> {
        ManyProgressBar { size, ranges }
    }
}

impl Widget for ManyProgressBar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let ManyProgressBar { size, ranges } = self;
        let available_size = ui.available_size_before_wrap();
        let (rect, resp) = ui.allocate_exact_size(vec2(available_size.x, size.y), Sense::hover());
        let visuals = &ui.style().visuals;
        ui.painter()
            .rect(rect, 10., visuals.extreme_bg_color, Stroke::NONE);
        for &(from, to, important) in ranges {
            let mut rounding = Rounding::same(10.);
            if from != 0. {
                rounding.nw = 0.;
                rounding.sw = 0.;
            }
            if to != 1. {
                rounding.ne = 0.;
                rounding.se = 0.;
            }
            ui.painter().rect(
                Rect::from_two_pos(
                    pos2(rect.left() + rect.width() * from, rect.top()),
                    pos2(rect.left() + rect.width() * to, rect.bottom()),
                ),
                rounding,
                if important {
                    Color32::RED
                } else {
                    visuals.selection.bg_fill
                },
                Stroke::NONE,
            );
        }
        resp
    }
}
