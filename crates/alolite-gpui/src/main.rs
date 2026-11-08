use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, Context, FontWeight, SharedString,
    Window, WindowBounds, WindowOptions,
};

struct AloLiteWindow {
    title: SharedString,
}

impl Render for AloLiteWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x18181b))
            .text_color(rgb(0xf4f4f5))
            .justify_center()
            .items_center()
            .gap_4()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child(self.title.clone()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xa1a1aa))
                    .child("Phase 0 Spike: GPU Rendered Window on Wayland/X11 (wgpu)"),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .px_3()
                            .py_1p5()
                            .rounded_md()
                            .bg(rgb(0x27272a))
                            .border_1()
                            .border_color(rgb(0x3f3f46))
                            .text_xs()
                            .child("Zero Electron · Zero WebKit"),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_1p5()
                            .rounded_md()
                            .bg(rgb(0x15803d))
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .child("1 Process · Direct GPU"),
                    ),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(720.0), px(480.0)),
                cx,
            ))),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("AloLite (GPUI Experiment)".into()),
                appears_transparent: false,
                traffic_light_position: None,
            }),
            ..Default::default()
        };

        cx.open_window(options, |_window, _cx| {
            _cx.new(|_cx| AloLiteWindow {
                title: "AloLite · GPUI Native Client".into(),
            })
        })
        .unwrap();
    });
}
