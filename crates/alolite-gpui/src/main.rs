mod input;

use gpui::{
    actions, div, prelude::*, px, rgb, size, App, Application, Bounds, Context, Entity,
    FocusHandle, Focusable, FontWeight, KeyBinding, Keystroke, MouseButton, MouseUpEvent, Window,
    WindowBounds, WindowOptions,
};
use input::{bind_default_input_keys, TextInput};

actions!(gate1_app, [Quit]);

struct Gate1Window {
    text_input: Entity<TextInput>,
    recent_keystrokes: Vec<Keystroke>,
    focus_handle: FocusHandle,
}

impl Focusable for Gate1Window {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Gate1Window {
    fn on_reset_click(&mut self, _: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.recent_keystrokes.clear();
        self.text_input
            .update(cx, |text_input, cx| text_input.reset(window, cx));
        cx.notify();
    }
}

impl Render for Gate1Window {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let input_read = self.text_input.read(cx);
        let text_len = input_read.content.chars().count();
        let utf16_len = input_read.content.encode_utf16().count();
        let bytes_len = input_read.content.len();
        let marked_info = match &input_read.marked_range {
            Some(r) => format!("Active ({:?}..{:?})", r.start, r.end),
            None => "Inactive (Idle)".to_string(),
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x09090b)) // zinc-950
            .text_color(rgb(0xf4f4f5)) // zinc-100
            .p(px(24.))
            .gap(px(16.))
            .track_focus(&self.focus_handle(cx))
            // Header
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0x38bdf8)) // sky-400
                                    .child("AloLite · GPUI Spike — Gate 1 (Vietnamese IME Test)"),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(rgb(0x27272a))
                                    .border_1()
                                    .border_color(rgb(0x3f3f46))
                                    .text_xs()
                                    .text_color(rgb(0xa1a1aa))
                                    .child(format!("Layout: {}", cx.keyboard_layout().name())),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0xa1a1aa))
                            .child("Mục tiêu: Gõ tiếng Việt bằng fcitx5 / ibus trên Wayland và quan sát xem có bị nhảy chữ, mất dấu hay double ký tự."),
                    ),
            )
            // Target Test Prompt Banner
            .child(
                div()
                    .flex()
                    .flex_col()
                    .p(px(12.))
                    .rounded_lg()
                    .bg(rgb(0x18181b))
                    .border_1()
                    .border_color(rgb(0x27272a))
                    .gap(px(6.))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xfacc15)) // yellow-400
                            .child("Câu mẫu cần test (gõ TELEX / VNI):"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0xffffff))
                            .child("Cộng hòa Xã hội Chủ nghĩa Việt Nam — Độc lập Tự do Hạnh phúc — 123456789"),
                    ),
            )
            // Input Area + Reset Button
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xd4d4d8))
                                    .child("Khung nhập văn bản (TextInput):"),
                            )
                            .child(
                                div()
                                    .cursor_pointer()
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0xd97706)) // amber-600
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xffffff))
                                    .child("Xóa làm lại (Reset)")
                                    .on_mouse_up(MouseButton::Left, cx.listener(Self::on_reset_click)),
                            ),
                    )
                    .child(self.text_input.clone()),
            )
            // Real-time diagnostics bar
            .child(
                div()
                    .flex()
                    .gap(px(12.))
                    .p(px(8.))
                    .rounded_md()
                    .bg(rgb(0x18181b))
                    .border_1()
                    .border_color(rgb(0x27272a))
                    .text_xs()
                    .text_color(rgb(0xa1a1aa))
                    .child(format!("Ký tự: {}", text_len))
                    .child(format!("UTF-16: {}", utf16_len))
                    .child(format!("Bytes: {}", bytes_len))
                    .child(format!("IME Composition: {}", marked_info)),
            )
            // Keystrokes Inspector
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(px(100.))
                    .p(px(10.))
                    .rounded_md()
                    .bg(rgb(0x18181b))
                    .border_1()
                    .border_color(rgb(0x27272a))
                    .gap(px(4.))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0x71717a))
                            .child("Nhật ký 12 sự kiện phím thô gần nhất (Keystroke Log):"),
                    )
                    .children(self.recent_keystrokes.iter().rev().take(12).map(|ks| {
                        div()
                            .text_xs()
                            .font_family("monospace")
                            .text_color(rgb(0x38bdf8))
                            .child(format!(
                                "▸ {} {}",
                                ks.unparse(),
                                if let Some(key_char) = ks.key_char.as_ref() {
                                    format!("char={:?}", key_char)
                                } else {
                                    "".to_string()
                                }
                            ))
                    })),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        bind_default_input_keys(cx);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([
            KeyBinding::new("ctrl-q", Quit, None),
            KeyBinding::new("cmd-q", Quit, None),
        ]);

        let bounds = Bounds::centered(None, size(px(760.0), px(560.0)), cx);

        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("AloLite · GPUI Spike (Gate 1 IME)".into()),
                        appears_transparent: false,
                        traffic_light_position: None,
                    }),
                    ..Default::default()
                },
                |_, cx| {
                    let text_input = cx.new(|cx| {
                        TextInput::new(cx, "Nhấp vào đây và gõ thử câu tiếng Việt bên trên...")
                    });
                    cx.new(|cx| Gate1Window {
                        text_input,
                        recent_keystrokes: Vec::new(),
                        focus_handle: cx.focus_handle(),
                    })
                },
            )
            .unwrap();

        let view = window.update(cx, |_, _, cx| cx.entity()).unwrap();
        cx.observe_keystrokes(move |ev, _, cx| {
            view.update(cx, |view, cx| {
                view.recent_keystrokes.push(ev.keystroke.clone());
                cx.notify();
            })
        })
        .detach();

        window
            .update(cx, |view, window, cx| {
                let handle = view.text_input.read(cx).focus_handle.clone();
                window.focus(&handle);
            })
            .unwrap();
    });
}
