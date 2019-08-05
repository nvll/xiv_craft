mod support;
use crate::config::{self, write_config};
use crate::macros::MacroFile;
use crate::task::{MaterialCount, Task};
use failure::Error;
use imgui::{im_str, ImGui, ImGuiCond, ImGuiInputTextFlags, ImString};
use imgui_winit_support;
use std::cmp::{max, min};
use std::time::Instant;
use support::button;

#[derive(Debug)]
struct UiState {
    add_clicked: bool,
    search_str: ImString,
    search_job: i32,
    macro_labels: Vec<ImString>,
    job_labels: Vec<ImString>,
    tasks_to_remove: Vec<usize>,
    return_tasks: bool,
}

impl Default for UiState {
    fn default() -> UiState {
        UiState {
            add_clicked: false,
            search_str: ImString::with_capacity(128),
            search_job: 0,
            macro_labels: Vec::new(),
            job_labels: xiv::JOBS.iter().map(|&j| ImString::new(j)).collect(),
            tasks_to_remove: Vec::new(),
            return_tasks: false,
        }
    }
}

const TASK_W: f32 = 400.0;
const TASK_H: f32 = 600.0;
const CONFIG_W: f32 = TASK_W;
const CONFIG_H: f32 = TASK_H;
const PADDING_W: f32 = 10.0;
const PADDING_H: f32 = 10.0;
const TOTAL_WIDTH: f32 = TASK_W + CONFIG_W + (PADDING_W * 3.0);
const TOTAL_HEIGHT: f32 = TASK_H + (PADDING_H * 2.0);

fn check_state_values(state: &mut UiState, tasks: &mut Vec<Task>) {
    // Due to borrow semantics, deferring the task remove to outside the iterator
    // borrow is necessary.
    if !state.tasks_to_remove.is_empty() {
        for task_id in &state.tasks_to_remove {
            tasks.remove(*task_id);
        }
        state.tasks_to_remove.clear();
    }
}

fn draw_ui<'a>(ui: &imgui::Ui<'a>, cfg: &mut config::Config, mut state: &mut UiState) -> bool {
    // Ensure our state is in a good ... state.
    check_state_values(&mut state, &mut cfg.tasks);
    if state.add_clicked {
        // Search for the recipe via XIVAPI. If we find it, create a backing task for it and
        // add it to our tasks.
        match xivapi::get_recipe_for_job(state.search_str.to_str(), state.search_job as u32) {
            Ok(v) => {
                log::trace!("recipe result is: {:#?}", v);
                if let Some(recipe) = v {
                    let task = Task {
                        quantity: 1,
                        is_collectable: false,
                        ignore_mat_quality: true,
                        // Initialize the material qualities to be NQ for everything
                        mat_quality: recipe
                            .mats
                            .iter()
                            .map(|m| MaterialCount { nq: m.count, hq: 0 })
                            .collect(),
                        recipe,
                        macro_id: 0,
                    };
                    cfg.tasks.push(task);
                }
            }
            Err(e) => println!("Error fetching recipe: {}", e.to_string()),
        }

        state.add_clicked = false;
    }

    // Left side window
    ui.window(im_str!("Talan"))
        .size((TASK_W, TASK_H), ImGuiCond::Always)
        .position((PADDING_W, PADDING_H), ImGuiCond::FirstUseEver)
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .build(|| {
            // Jobs for the combo box. Can't be constant due to unknown size at compile time.
            ui.with_item_width(60.0, || {
                support::combobox(
                    &ui,
                    im_str!("Job"),
                    &state.job_labels,
                    &mut state.search_job,
                );
            });
            ui.same_line(0.0);
            // Both pressing enter in the item textbox and pressing the add button should
            // register a recipe lookup.
            ui.with_item_width(200.0, || {
                if ui
                    .input_text(im_str!("Item"), &mut state.search_str)
                    .flags(
                        ImGuiInputTextFlags::EnterReturnsTrue | ImGuiInputTextFlags::AutoSelectAll,
                    )
                    .build()
                {
                    state.add_clicked = true;
                }
                ui.same_line(0.0);
                if ui.button(im_str!("Add"), (0.0, 0.0)) {
                    state.add_clicked = true;
                }
            });

            // Both Tasks and their materials are enumerated so we can generate unique
            // UI ids for widgets and prevent any sort of UI clash.
            for (task_id, task) in &mut cfg.tasks.iter_mut().enumerate() {
                ui.push_id(task_id as i32);
                // header should be closeable
                let header_name = ImString::new(format!(
                    "[{}] {}x {} {}",
                    xiv::JOBS[task.recipe.job as usize],
                    task.quantity,
                    task.recipe.name.clone(),
                    if task.is_collectable {
                        "(Collectable)"
                    } else {
                        ""
                    }
                ));
                if ui
                    .collapsing_header(&header_name)
                    .default_open(true)
                    .build()
                {
                    ui.checkbox(
                        im_str!("Use materials of any quality"),
                        &mut task.ignore_mat_quality,
                    );
                    for (i, (mat, qual)) in task
                        .recipe
                        .mats
                        .iter()
                        .zip(task.mat_quality.iter_mut())
                        .enumerate()
                    {
                        ui.push_id(i as i32);
                        if task.ignore_mat_quality {
                            // Create a quick label string for the material
                            ui.text(format!("{}x {}", mat.count, mat.name));
                        } else {
                            // Otherwise we need to convert some numerical values to strings,
                            // then feed them into the widgets. This seems like it should
                            // thrash like crazy, but thankfully it's 2019 and processors
                            // are fast?
                            let mut nq_imstr = ImString::new(qual.nq.to_string());
                            ui.text(&ImString::new(mat.name.clone()));
                            ui.with_item_width(25.0, || {
                                ui.input_text(im_str!("NQ"), &mut nq_imstr)
                                    .flags(ImGuiInputTextFlags::ReadOnly)
                                    .build();
                            });
                            ui.same_line(0.0);
                            ui.with_item_width(75.0, || {
                                // Use a temp to deal with imgui only allowing i32
                                let mut hq: i32 = qual.hq as i32;
                                if ui.input_int(im_str!("HQ"), &mut hq).build() {
                                    qual.hq = min(max(0, hq as u32), mat.count);
                                    qual.nq = mat.count - qual.hq;
                                }
                            });
                        }
                        ui.pop_id();
                    }

                    ui.with_item_width(75.0, || {
                        if ui.input_int(im_str!("Count"), &mut task.quantity).build() {
                            task.quantity = max(1, task.quantity);
                        }
                        ui.same_line(0.0);
                        ui.checkbox(im_str!("Collectable"), &mut task.is_collectable);
                    });
                    support::combobox(
                        ui,
                        im_str!("Macro"),
                        &state.macro_labels,
                        &mut task.macro_id,
                    );

                    if support::button(ui, "Delete Task") {
                        state.tasks_to_remove.push(task_id);
                    }
                }
                ui.pop_id();
            }

            ui.separator();
            // Only show the craft button if we have tasks added
            if !cfg.tasks.is_empty() && button(ui, "Craft Tasks") {
                if write_config(cfg).is_err() {
                    log::error!("failed to write config");
                }
                state.return_tasks = true;
            }
        });
    // Right side window
    ui.window(im_str!("Configuration"))
        .size((CONFIG_W, CONFIG_H), ImGuiCond::FirstUseEver)
        .position((TASK_W + (PADDING_W * 2.0), PADDING_H), ImGuiCond::Always)
        .movable(false)
        .collapsible(false)
        .resizable(false)
        .build(|| {
            if ui
                .collapsing_header(im_str!("Gear Sets"))
                .default_open(true)
                .build()
            {
                ui.with_item_width(70.0, || {
                    if ui.input_int(im_str!("Carpenter"), &mut cfg.gear[0]).build() {
                        cfg.gear[0] = max(cfg.gear[0], 0);
                    }
                    if ui
                        .input_int(im_str!("Blacksmith"), &mut cfg.gear[1])
                        .build()
                    {
                        cfg.gear[1] = max(cfg.gear[1], 0);
                    }
                    if ui.input_int(im_str!("Armorer"), &mut cfg.gear[2]).build() {
                        cfg.gear[2] = max(cfg.gear[2], 0);
                    }
                    if ui.input_int(im_str!("Goldsmith"), &mut cfg.gear[3]).build() {
                        cfg.gear[3] = max(cfg.gear[3], 0);
                    }
                    if ui
                        .input_int(im_str!("Leatherworker"), &mut cfg.gear[4])
                        .build()
                    {
                        cfg.gear[4] = max(cfg.gear[4], 0);
                    }
                    if ui.input_int(im_str!("Weaver"), &mut cfg.gear[5]).build() {
                        cfg.gear[5] = max(cfg.gear[5], 0);
                    }
                    if ui.input_int(im_str!("Alchemist"), &mut cfg.gear[6]).build() {
                        cfg.gear[6] = max(cfg.gear[6], 0);
                    }
                    if ui
                        .input_int(im_str!("Culinarian"), &mut cfg.gear[7])
                        .build()
                    {
                        cfg.gear[7] = max(cfg.gear[7], 0);
                    }
                    if ui
                        .input_int(
                            im_str!("Non-DoH set (for Collectable safety)"),
                            &mut cfg.non_doh_gear,
                        )
                        .build()
                    {
                        cfg.non_doh_gear = max(cfg.non_doh_gear, 0);
                    }
                });
            }
            if ui
                .collapsing_header(im_str!("Options"))
                .default_open(true)
                .build()
            {
                ui.checkbox(
                    im_str!("Reload task list at start"),
                    &mut cfg.options.reload_tasks,
                );
                if ui.is_item_hovered() {
                    ui.tooltip_text(
                        "Tasks will be saved when tasks are started, or the config is saved",
                    );
                }
                ui.checkbox(
                    im_str!("Use slower menu navigation"),
                    &mut cfg.options.use_slow_navigation,
                );
                if ui.is_item_hovered() {
                    ui.tooltip_text(
                        "Use this option if you have a lower (<30) fps or high latency",
                    );
                }
            };
            if ui.small_button(im_str!("Save changes")) && write_config(cfg).is_err() {
                println!("Error writing config :(")
            }
        });

    // Return if we're supposed to
    state.return_tasks
}

pub fn start(mut cfg: &mut config::Config, macros: &[MacroFile]) -> Result<bool, Error> {
    use glium::glutin;
    use glium::{Display, Surface};
    use imgui_glium_renderer::Renderer;

    let mut events_loop = glutin::EventsLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let builder = glutin::WindowBuilder::new()
        .with_title("Talan")
        .with_dimensions(glutin::dpi::LogicalSize::new(
            f64::from(TOTAL_WIDTH),
            f64::from(TOTAL_HEIGHT),
        ));
    let display = Display::new(builder, context, &events_loop).unwrap();
    let window = display.gl_window();
    let hidpi_factor = window.get_hidpi_factor().round();

    let mut imgui = ImGui::init();
    imgui.set_ini_filename(None); // Without this, imgui will save a .ini in PWD
    support::set_style(&mut imgui);
    support::set_fonts(&mut imgui, hidpi_factor);

    let mut renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");
    imgui_winit_support::configure_keys(&mut imgui);
    let mut last_frame = Instant::now();
    let mut quit = false;

    // Our initial state is the default for the gui, along with the macros we find while
    // scanning.
    let mut ui_state = UiState::default();
    for m in macros {
        ui_state.macro_labels.push(ImString::new(m.name.clone()));
    }

    loop {
        events_loop.poll_events(|event| {
            use glium::glutin::{Event, WindowEvent::CloseRequested};

            imgui_winit_support::handle_event(
                &mut imgui,
                &event,
                window.get_hidpi_factor(),
                hidpi_factor,
            );

            if let Event::WindowEvent { event, .. } = event {
                if event == CloseRequested {
                    quit = true;
                }
            }
        });

        let now = Instant::now();
        let delta = now - last_frame;
        let delta_s = delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;
        last_frame = now;

        let frame_size = imgui_winit_support::get_frame_size(&window, hidpi_factor).unwrap();

        let ui = imgui.frame(frame_size, delta_s);
        let result = draw_ui(&ui, &mut cfg, &mut ui_state);
        if result {
            quit = true;
        }

        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);
        if !quit {
            renderer.render(&mut target, ui).expect("Rendering failed");
        }
        target.finish().unwrap();
        if quit {
            return Ok(result);
        }
    }
}
