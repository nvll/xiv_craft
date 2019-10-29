use glium::glutin::{self, Event, WindowEvent};
use glium::{Display, Surface};
use imgui::{Context, FontConfig, FontGlyphRanges, FontSource, StyleColor, Ui};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::time::Instant;

mod clipboard;

pub struct System {
    pub events_loop: glutin::EventsLoop,
    pub display: glium::Display,
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
    pub font_size: f32,
}

const FONT_SIZE: f64 = 16.0;

// This method is copied from Imgui-rs's v0.1.0 example support code
// with the following modifications:
// - Talan's visual style is configured.
// - Font was changed.
pub fn init(width: f64, height: f64, title: &str) -> System {
    let title = match title.rfind('/') {
        Some(idx) => title.split_at(idx + 1).1,
        None => title,
    };
    let events_loop = glutin::EventsLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let builder = glutin::WindowBuilder::new()
        .with_title(title.to_owned())
        .with_dimensions(glutin::dpi::LogicalSize::new(width, height));
    let display =
        Display::new(builder, context, &events_loop).expect("Failed to initialize display");

    let mut imgui = Context::create();
    imgui.set_ini_filename(None);

    if let Some(backend) = clipboard::init() {
        imgui.set_clipboard_backend(Box::new(backend));
    } else {
        eprintln!("Failed to initialize clipboard");
    }

    let mut platform = WinitPlatform::init(&mut imgui);
    {
        let gl_window = display.gl_window();
        let window = gl_window.window();
        platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Rounded);
    }

    let hidpi_factor = platform.hidpi_factor();
    let font_size = (FONT_SIZE * hidpi_factor) as f32;
    imgui.fonts().add_font(&[
        FontSource::TtfData {
            data: include_bytes!("DroidSans.ttf"),
            size_pixels: font_size,
            config: Some(FontConfig {
                rasterizer_multiply: 1.75,
                glyph_ranges: FontGlyphRanges::default(),
                ..FontConfig::default()
            }),
        },
        FontSource::DefaultFontData {
            config: Some(FontConfig {
                size_pixels: font_size,
                ..FontConfig::default()
            }),
        },
    ]);

    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
    let mut style = imgui.style_mut();
    style.use_dark_colors();
    style.frame_border_size = 1.0;
    style.frame_rounding = 0.0;
    style.window_rounding = 0.0;
    style.colors[StyleColor::TitleBg as usize] = style.colors[StyleColor::TitleBgActive as usize];
    style.colors[StyleColor::PlotHistogram as usize] = style.colors[StyleColor::Button as usize];

    let renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");

    System {
        events_loop,
        display,
        imgui,
        platform,
        renderer,
        font_size,
    }
}

impl System {
    pub fn main_loop<F: FnMut(&mut bool, &mut Ui)>(self, mut run_ui: F) {
        let System {
            mut events_loop,
            display,
            mut imgui,
            mut platform,
            mut renderer,
            ..
        } = self;
        let gl_window = display.gl_window();
        let window = gl_window.window();
        let mut last_frame = Instant::now();
        let mut run = true;

        while run {
            events_loop.poll_events(|event| {
                platform.handle_event(imgui.io_mut(), &window, &event);

                if let Event::WindowEvent { event, .. } = event {
                    if let WindowEvent::CloseRequested = event {
                        run = false;
                    }
                }
            });

            let io = imgui.io_mut();
            platform
                .prepare_frame(io, &window)
                .expect("Failed to start frame");
            last_frame = io.update_delta_time(last_frame);
            let mut ui = imgui.frame();
            run_ui(&mut run, &mut ui);

            let mut target = display.draw();
            target.clear_color_srgb(1.0, 1.0, 1.0, 1.0);
            platform.prepare_render(&ui, &window);
            let draw_data = ui.render();
            renderer
                .render(&mut target, draw_data)
                .expect("Rendering failed");
            target.finish().expect("Failed to swap buffers");
        }
    }
}

pub fn set_talan_style(style: &mut imgui::Style) {
    // Set all windows / widgets to rectangles
    style.child_rounding = 0.0;
    style.popup_rounding = 0.0;
    style.frame_rounding = 0.0;
    style.window_rounding = 0.0;
    style.frame_border_size = 1.0;

    // This style is adapted from the light style in imgui_draw.cpp
    style.colors[StyleColor::Text as usize] = [0.00, 0.00, 0.00, 1.00];
    style.colors[StyleColor::TextDisabled as usize] = [1.60, 1.60, 0.60, 1.00];
    style.colors[StyleColor::WindowBg as usize] = [0.94, 0.94, 0.94, 1.00];
    style.colors[StyleColor::ChildBg as usize] = [0.00, 0.00, 0.00, 0.00];
    style.colors[StyleColor::PopupBg as usize] = [1.00, 1.00, 1.00, 0.98];
    style.colors[StyleColor::Border as usize] = [0.00, 0.00, 0.00, 0.00];
    style.colors[StyleColor::BorderShadow as usize] = [0.00, 0.00, 0.00, 0.00];
    style.colors[StyleColor::FrameBg as usize] = [1.00, 1.00, 1.00, 1.00];
    style.colors[StyleColor::FrameBgHovered as usize] = [0.26, 0.59, 0.98, 0.40];
    style.colors[StyleColor::FrameBgActive as usize] = [0.26, 0.59, 0.98, 0.67];
    style.colors[StyleColor::TitleBg as usize] = [0.96, 0.96, 0.96, 1.00];
    style.colors[StyleColor::TitleBgActive as usize] = [0.82, 0.82, 0.82, 1.00];
    style.colors[StyleColor::TitleBgCollapsed as usize] = [1.00, 1.00, 1.00, 0.51];
    style.colors[StyleColor::MenuBarBg as usize] = [0.86, 0.86, 0.86, 1.00];
    style.colors[StyleColor::ScrollbarBg as usize] = [0.98, 0.98, 0.98, 0.53];
    style.colors[StyleColor::ScrollbarGrab as usize] = [0.69, 0.69, 0.69, 0.80];
    style.colors[StyleColor::ScrollbarGrabHovered as usize] = [0.49, 0.49, 0.49, 0.80];
    style.colors[StyleColor::ScrollbarGrabActive as usize] = [0.49, 0.49, 0.49, 1.00];
    style.colors[StyleColor::CheckMark as usize] = [0.26, 0.59, 0.98, 1.00];
    style.colors[StyleColor::SliderGrab as usize] = [0.26, 0.59, 0.98, 0.78];
    style.colors[StyleColor::SliderGrabActive as usize] = [0.46, 0.54, 0.80, 0.60];
    style.colors[StyleColor::Button as usize] = [0.26, 0.59, 0.98, 0.40];
    style.colors[StyleColor::ButtonHovered as usize] = [0.26, 0.59, 0.98, 1.00];
    style.colors[StyleColor::ButtonActive as usize] = [0.06, 0.53, 0.98, 1.00];
    style.colors[StyleColor::Header as usize] = [0.26, 0.59, 0.98, 0.31];
    style.colors[StyleColor::HeaderHovered as usize] = [0.26, 0.59, 0.98, 0.31];
    //style.colors[StyleColor::HeaderHovered as usize] = [0.26, 0.59, 0.98, 0.80];
    style.colors[StyleColor::HeaderActive as usize] = [0.26, 0.59, 0.98, 1.00];
    style.colors[StyleColor::Separator as usize] = [0.39, 0.39, 0.39, 1.00];
    style.colors[StyleColor::SeparatorHovered as usize] = [0.14, 0.44, 0.80, 0.78];
    style.colors[StyleColor::SeparatorActive as usize] = [0.14, 0.44, 0.80, 1.00];
    style.colors[StyleColor::ResizeGrip as usize] = [0.80, 0.80, 0.80, 0.56];
    style.colors[StyleColor::ResizeGripHovered as usize] = [0.26, 0.59, 0.98, 0.67];
    style.colors[StyleColor::ResizeGripActive as usize] = [0.26, 0.59, 0.98, 0.95];
    style.colors[StyleColor::PlotLines as usize] = [0.39, 0.39, 0.39, 1.00];
    style.colors[StyleColor::PlotLinesHovered as usize] = [1.00, 0.43, 0.35, 1.00];
    style.colors[StyleColor::PlotHistogram as usize] = [0.26, 0.59, 0.98, 0.40];
    style.colors[StyleColor::PlotHistogramHovered as usize] = [1.00, 0.45, 0.00, 1.00];
    style.colors[StyleColor::TextSelectedBg as usize] = [0.26, 0.59, 0.98, 0.35];
    style.colors[StyleColor::DragDropTarget as usize] = [0.26, 0.59, 0.98, 0.95];
    style.colors[StyleColor::NavHighlight as usize] =
        style.colors[StyleColor::HeaderHovered as usize];
    style.colors[StyleColor::NavWindowingHighlight as usize] = [0.70, 0.70, 0.70, 0.70];
    style.colors[StyleColor::NavWindowingDimBg as usize] = [0.20, 0.20, 0.20, 0.20];
    style.colors[StyleColor::ModalWindowDimBg as usize] = [0.20, 0.20, 0.20, 0.35];
}
