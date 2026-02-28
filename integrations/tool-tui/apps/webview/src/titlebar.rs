use gpui::{div, prelude::*, rgb};

#[derive(IntoElement)]
pub struct DxTitleBar {
    url_label: String,
}

impl DxTitleBar {
    pub fn new(url_label: impl Into<String>) -> Self {
        Self {
            url_label: url_label.into(),
        }
    }

    fn pill(label: &'static str) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .text_xs()
            .text_color(rgb(0x888888))
            .bg(rgb(0x1a1a1a))
            .rounded_md()
            .child(label)
    }
}

impl RenderOnce for DxTitleBar {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_3()
            .child(div().text_sm().text_color(rgb(0xa4a4a4)).child("DX"))
            .child(Self::pill("File"))
            .child(Self::pill("Edit"))
            .child(Self::pill("View"))
            .child(div().text_sm().text_color(rgb(0x747474)).child(self.url_label))
    }
}
