use crate::config::{self, write_config};
use crate::macros::MacroFile;
use crate::task::{MaterialCount, Task};
use failure::Error;
use gui_support;
use imgui::*;
use std::cmp::max;

const WINDOW_W: f32 = 400.0;
const WINDOW_H: f32 = 700.0;
const HEADER_H: f32 = 25.0;
const FOOTER_H: f32 = 25.0;

#[derive(Debug)]
struct UiState {
    show_config_window: bool,
    craft_button_clicked: bool,
    add_task_button_clicked: bool,
    search_str: ImString,
    search_job: i32,
    exit_gui: bool,
    task_to_remove: Option<usize>,
}

impl Default for UiState {
    fn default() -> UiState {
        UiState {
            show_config_window: false,
            add_task_button_clicked: false,
            craft_button_clicked: false,
            search_str: ImString::with_capacity(128),
            search_job: 0,
            exit_gui: false,
            task_to_remove: None,
        }
    }
}

pub struct Gui {
    state: UiState,
    macro_labels: Vec<ImString>,
    job_labels: Vec<ImString>,
}

impl<'a> Gui {
    pub fn new(macros: &'a [MacroFile]) -> Gui {
        Gui {
            state: UiState::default(),
            macro_labels: macros
                .iter()
                .map(|m| ImString::new(m.name.clone()))
                .collect(),
            job_labels: xiv::JOBS.iter().map(|&j| ImString::new(j)).collect(),
        }
    }

    pub fn start(&mut self, mut config: &mut config::Config) -> Result<bool, Error> {
        let system = gui_support::init(WINDOW_W as f64, WINDOW_H as f64, "Talan");

        // Due to the way borrowing and closures work, most of the rendering impl
        // borrow inner members of our GUI state and are otherwise not methods.
        system.main_loop(|run, ui| {
            if Gui::draw_main_window(
                &ui,
                &mut config,
                &mut self.state,
                &self.macro_labels[..],
                &self.job_labels[..],
            ) {
                *run = false;
            }

            if self.state.show_config_window {
                Gui::draw_config_window(&ui, &mut config, &mut self.state);
            }

            if let Some(id) = self.state.task_to_remove {
                config.tasks.remove(id);
                self.state.task_to_remove = None;
            }

            if self.state.add_task_button_clicked {
                // Search for the recipe via XIVAPI. If we find it, create a backing task for it and
                // add it to our tasks.
                match xivapi::get_recipe_for_job(
                    self.state.search_str.to_str(),
                    self.state.search_job as u32,
                ) {
                    Ok(v) => {
                        log::trace!("recipe result is: {:#?}", v);
                        if let Some(recipe) = v {
                            let task = Task {
                                quantity: 1,
                                is_collectable: false,
                                use_any_mats: true,
                                // Initialize the material qualities to be NQ for everything
                                mat_quality: recipe
                                    .mats
                                    .iter()
                                    .map(|m| MaterialCount { nq: m.count, hq: 0 })
                                    .collect(),
                                recipe,
                                macro_id: 0,
                            };
                            config.tasks.push(task);
                        }
                    }
                    Err(e) => println!("Error fetching recipe: {}", e.to_string()),
                }

                self.state.add_task_button_clicked = false;
            }
        });

        Ok(self.state.craft_button_clicked)
    }

    fn draw_main_window<'b>(
        ui: &imgui::Ui<'b>,
        config: &mut config::Config,
        state: &mut UiState,
        macros: &[ImString],
        jobs: &[ImString],
    ) -> bool {
        let mut menu_height: f32 = 0.0;
        ui.window(im_str!("Talan"))
            .size([WINDOW_W, WINDOW_H], Condition::FirstUseEver)
            .position([0.0, menu_height], Condition::FirstUseEver)
            .scroll_bar(false)
            .title_bar(false)
            .movable(false)
            .resizable(false)
            .collapsible(false)
            .menu_bar(true)
            .build(|| {
                // Attaching the menu to the main window seems to make calculating
                // offsets easier than if it attached to the window context itself.
                ui.menu_bar(|| {
                    ui.menu(im_str!("File")).build(|| {
                        ui.menu_item(im_str!("Preferences"))
                            .selected(&mut state.show_config_window)
                            .build();
                        ui.separator();
                        ui.menu_item(im_str!("Quit"))
                            .selected(&mut state.exit_gui)
                            .build();
                    });
                    menu_height = ui.get_window_size()[1];
                });
                // THe header frame contains our search box, the job selector,
                // and the 'Add Task' button.
                ui.child_frame(im_str!("Header"), [0.0, HEADER_H])
                    .build(|| {
                        {
                            let _width = ui.push_item_width(60.0);
                            gui_support::combobox(ui, im_str!("Job"), &mut state.search_job, &jobs);
                        }
                        ui.same_line(0.0);
                        // Both pressing enter in the item textbox and pressing the add button should
                        // register a recipe lookup.
                        {
                            let _width = ui.push_item_width(200.0);
                            if ui
                                .input_text(im_str!("Item"), &mut state.search_str)
                                .flags(
                                    ImGuiInputTextFlags::EnterReturnsTrue
                                        | ImGuiInputTextFlags::AutoSelectAll,
                                )
                                .build()
                            {
                                state.add_task_button_clicked = true;
                            }
                            ui.same_line(0.0);
                            if ui.button(im_str!("Add"), [0.0, 0.0]) {
                                state.add_task_button_clicked = true;
                            }
                        }
                    });
                ui.spacing();
                ui.child_frame(
                    im_str!("Task list"),
                    [0.0, WINDOW_H - HEADER_H - FOOTER_H - menu_height],
                )
                .build(|| {
                    // Both Tasks and their materials are enumerated so we can generate unique
                    // UI ids for widgets and prevent any sort of UI clash.
                    for (ui_id, mut t) in &mut config.tasks.iter_mut().enumerate() {
                        if !Gui::draw_task(ui, ui_id as i32, &mut t, macros) {
                            state.task_to_remove = Some(ui_id);
                        }
                    }
                });
                ui.spacing();
                ui.child_frame(im_str!("Craft Frame"), [0.0, FOOTER_H])
                    .build(|| {
                        // Only show the craft button if we have tasks added
                        if !config.tasks.is_empty() && ui.button(im_str!("Craft Tasks"), [0.0, 0.0])
                        {
                            if write_config(config).is_err() {
                                log::error!("failed to write config");
                            }
                            state.exit_gui = true;
                            state.craft_button_clicked = true;
                        }
                    });
            });

        // If we return a |true| value the main_loop will know to exit the run loop.
        state.exit_gui
    }

    fn draw_task<'b>(ui: &imgui::Ui<'b>, ui_id: i32, task: &mut Task, macros: &[ImString]) -> bool {
        ui.push_id(ui_id);
        // header should be closeable
        let header_name = ImString::new(format!(
            "[{} {}] {}x {} {}",
            xiv::JOBS[task.recipe.job as usize],
            task.recipe.level,
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
            ui.text(format!(
                "{} Durability  {} Difficulty  {} Quality",
                task.recipe.durability, task.recipe.difficulty, task.recipe.quality
            ));
            ui.same_line(ui.get_window_size()[0] - 15.0);
            if ui.small_button(im_str!("x")) {
                ui.pop_id();
                return false;
            }
            ui.checkbox(
                im_str!("Use materials of any quality"),
                &mut task.use_any_mats,
            );

            // Draw material widgets, or just the checkbox if checked.
            for (i, (mat, qual)) in task
                .recipe
                .mats
                .iter()
                .zip(task.mat_quality.iter_mut())
                .enumerate()
            {
                ui.push_id(i as i32);
                if task.use_any_mats {
                    // Create a quick label string for the material
                    ui.text(format!("{}x {}", mat.count, mat.name));
                } else {
                    // Otherwise we need to convert some numerical values to strings,
                    // then feed them into the widgets. This seems like it should
                    // thrash like crazy, but thankfully it's 2019 and processors
                    // are fast?
                    let mut nq_imstr = ImString::new(qual.nq.to_string());
                    ui.text(&ImString::new(mat.name.clone()));
                    {
                        let _width = ui.push_item_width(25.0);
                        ui.input_text(im_str!("NQ"), &mut nq_imstr)
                            .flags(ImGuiInputTextFlags::ReadOnly)
                            .build();
                    };
                    ui.same_line(0.0);
                    {
                        let _width = ui.push_item_width(75.0);
                        // Use a temp to deal with imgui only allowing i32
                        let mut hq: i32 = qual.hq as i32;
                        if ui.input_int(im_str!("HQ"), &mut hq).build() {
                            // TODO: Material selection isn't fully implemented, so
                            // disable the HQ box
                            qual.hq = 0;
                            // qual.hq = min(max(0, hq as u32), mat.count);
                            // qual.nq = mat.count - qual.hq;
                        }
                    };
                }
                ui.pop_id();
            }
            {
                let _width = ui.push_item_width(75.0);
                if ui.input_int(im_str!("Count"), &mut task.quantity).build() {
                    task.quantity = max(1, task.quantity);
                }
                ui.same_line(0.0);
                ui.checkbox(im_str!("Collectable"), &mut task.is_collectable);
            };
            gui_support::combobox(ui, im_str!("Macro"), &mut task.macro_id, &macros);
        }
        ui.pop_id();

        true
    }

    fn draw_config_window(ui: &imgui::Ui, config: &mut config::Config, state: &mut UiState) {
        // Right side window
        ui.window(im_str!("Preferences"))
            .always_auto_resize(true)
            .opened(&mut state.show_config_window)
            .collapsible(false)
            .build(|| {
                if ui
                    .collapsing_header(im_str!("Gear Sets"))
                    .default_open(true)
                    .build()
                {
                    {
                        let _width = ui.push_item_width(80.0);
                        if ui
                            .input_int(im_str!("Carpenter"), &mut config.gear[0])
                            .build()
                        {
                            config.gear[0] = max(config.gear[0], 0);
                        }
                        if ui
                            .input_int(im_str!("Blacksmith"), &mut config.gear[1])
                            .build()
                        {
                            config.gear[1] = max(config.gear[1], 0);
                        }
                        if ui
                            .input_int(im_str!("Armorer"), &mut config.gear[2])
                            .build()
                        {
                            config.gear[2] = max(config.gear[2], 0);
                        }
                        if ui
                            .input_int(im_str!("Goldsmith"), &mut config.gear[3])
                            .build()
                        {
                            config.gear[3] = max(config.gear[3], 0);
                        }
                        if ui
                            .input_int(im_str!("Leatherworker"), &mut config.gear[4])
                            .build()
                        {
                            config.gear[4] = max(config.gear[4], 0);
                        }
                        if ui.input_int(im_str!("Weaver"), &mut config.gear[5]).build() {
                            config.gear[5] = max(config.gear[5], 0);
                        }
                        if ui
                            .input_int(im_str!("Alchemist"), &mut config.gear[6])
                            .build()
                        {
                            config.gear[6] = max(config.gear[6], 0);
                        }
                        if ui
                            .input_int(im_str!("Culinarian"), &mut config.gear[7])
                            .build()
                        {
                            config.gear[7] = max(config.gear[7], 0);
                        }
                        if ui
                            .input_int(
                                im_str!("Non-DoH set (for Collectable safety)"),
                                &mut config.non_doh_gear,
                            )
                            .build()
                        {
                            config.non_doh_gear = max(config.non_doh_gear, 0);
                        }
                    }
                }
                if ui
                    .collapsing_header(im_str!("Options"))
                    .default_open(true)
                    .build()
                {
                    ui.checkbox(
                        im_str!("Reload task list at start"),
                        &mut config.options.reload_tasks,
                    );
                    if ui.is_item_hovered() {
                        ui.tooltip_text(
                            "Tasks will be saved when tasks are started, or the config is saved",
                        );
                    }
                    ui.checkbox(
                        im_str!("Use slower menu navigation"),
                        &mut config.options.use_slow_navigation,
                    );
                    if ui.is_item_hovered() {
                        ui.tooltip_text(
                            "Use this option if you have a lower (<30) fps or high latency",
                        );
                    }
                };
                if ui.button(im_str!("Save changes"), [0.0, 0.0]) && write_config(config).is_err() {
                    log::error!("Error writing config :(")
                }
            });
    }
}
