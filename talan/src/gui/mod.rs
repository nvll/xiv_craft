use crate::config::{self, write_config};
use crate::macros::MacroFile;
use crate::rpc::{Request, Response};
use crate::task::{Status, Task};
use failure::Error;
use gui_support;
use imgui::*;
use std::cmp::{max, min};
use std::sync::mpsc::{Receiver, Sender};

// These represent a ratio compared to WINDOW_W and WINDOW_H
const ADD_TASKS_SIZE: [f32; 2] = [400.0, 125.0];
const ADD_TASKS_POSITION: [f32; 2] = [10.0, 30.0];
const TASK_LIST_SIZE: [f32; 2] = [400.0, 575.0];
const TASK_LIST_POSITION: [f32; 2] = [10.0, 165.0];
const CONFIGURATION_POSITION: [f32; 2] = [420.0, 30.0];
const CONFIGURATION_SIZE: [f32; 2] = [300.0, 500.0];

const WINDOW_W: f32 = CONFIGURATION_POSITION[0] + CONFIGURATION_SIZE[0] + 10.0;
const WINDOW_H: f32 = TASK_LIST_POSITION[1] + TASK_LIST_SIZE[1] + 10.0;

/// A type to represent any popup needed, whether positive or negative.
#[derive(Debug)]
struct ModalText {
    has_msg: bool,
    title: ImString,
    msg: ImString,
}

/// UiState tracks all the state specific to ImGui and any widget
/// events triggered by the user. All widgets that bind to data
/// will find their backing store here.
#[derive(Debug)]
struct UiState {
    // TODO: Convert some of these into enum values
    /// |true| if we're leaving the gui becaue the user pressed 'craft'.
    begin_crafts_selected: bool,
    /// |true| if a task set was sent to the worker thread.
    craft_started: bool,
    craft_status: Vec<Status>,
    craft_stopped: bool,
    waiting_to_stop: bool,
    /// Store for the Error / Message popup
    modal_popup: ModalText,
    // Whether a task add was triggered via the button or enter in the text box.
    add_task_button_clicked: bool,
    // The item search string.
    search_str: ImString,
    // The job dropdown selection.
    search_job: i32,
    // If we should leave the gui main loop.
    exit_gui: bool,
    // These are used to track a task that should be modified in the list.
    clear_task_list: bool,
    task_to_remove: Option<i32>,
    task_to_move: Option<(i32, i32)>, // index, offset (-1, +1)
    // |true| when a request has been sent to the xivapi worker thread.
    searching: bool,
}

impl Default for UiState {
    fn default() -> UiState {
        UiState {
            modal_popup: ModalText {
                has_msg: false,
                title: ImString::with_capacity(128),
                msg: ImString::with_capacity(512),
            },
            add_task_button_clicked: false,
            begin_crafts_selected: false,
            craft_started: false,
            craft_status: Vec::new(),
            craft_stopped: false,
            waiting_to_stop: false,
            search_str: ImString::with_capacity(128),
            search_job: 0,
            exit_gui: false,
            clear_task_list: false,
            task_to_remove: None,
            task_to_move: None,
            searching: false,
        }
    }
}

pub struct Gui<'a> {
    state: UiState,
    macro_labels: Vec<ImString>,
    job_labels: Vec<ImString>,
    rpc_tx: &'a Sender<Request>,
    rpc_rx: &'a Receiver<Response>,
    macros: &'a [MacroFile],
}

impl<'a, 'b> Gui<'a> {
    pub fn new(
        macros: &'a [MacroFile],
        rpc_tx: &'a Sender<Request>,
        rpc_rx: &'a Receiver<Response>,
    ) -> Gui<'a> {
        Gui {
            state: UiState::default(),
            macro_labels: macros
                .iter()
                .map(|m| ImString::new(m.name.clone()))
                .collect(),
            job_labels: xiv::JOBS.iter().map(|&j| ImString::new(j)).collect(),
            rpc_tx,
            rpc_rx,
            macros, // Temporary until macro rework
        }
    }

    pub fn start(&mut self, mut config: &mut config::Config) -> Result<bool, Error> {
        let system = gui_support::init(f64::from(WINDOW_W), f64::from(WINDOW_H), "Talan");

        // Due to the way borrowing and closures work, most of the rendering impl
        // borrow inner members of our GUI state and are otherwise not methods.
        system.main_loop(|run, ui| {
            // Render the menu, and handle any results of that.
            Gui::main_menu(&ui, &mut self.state);
            // If |Begin Crafting| was selected from the menu
            if self.state.begin_crafts_selected {
                if Gui::check_gear_sets(&mut self.state, &config) {
                    self.rpc_tx
                        .send(Request::Craft {
                            options: config.options,
                            tasks: config.tasks.clone(),
                            macros: self.macros.to_vec(),
                        })
                        .unwrap_or_else(|e| {
                            log::error!("rpc failed at line {}: {}", line!(), e.to_string())
                        });
                }
                self.state.begin_crafts_selected = false;
                self.state.craft_started = true;
            }

            // Once we've send a Request::Craft to the worker it will craft
            // items and periodicially send a Response::Status after finishing
            // each item. When the worker is done working it will send a
            // Response::EOW. This applies in all cases work stops, whether it
            // be due to a successful completion, |stop| being clicked, or other
            // error.
            if self.state.craft_started {
                if let Ok(resp) = self.rpc_rx.try_recv() {
                    match resp {
                        Response::Craft(status) => {
                            self.state.craft_status = status;
                        }
                        Response::EOW => {
                            self.state.waiting_to_stop = false;
                            self.state.craft_started = false;
                        }

                        _ => (),
                    }
                }
                Gui::progress_window(&ui, &mut self.state);
            }

            // If crafting was stopped then update the task list to reflect the
            // items that were finished in the interim.
            if self.state.craft_stopped {
                self.rpc_tx.send(Request::StopCrafting).unwrap_or_else(|e| {
                    log::error!("rpc failed at line {}: {}", line!(), e.to_string())
                });
                self.state.craft_stopped = false;
                self.state.waiting_to_stop = true;
            }

            // Clear out task lists before rendering the list, if the menu option
            // was selected.
            if self.state.clear_task_list {
                self.state.clear_task_list = false;
                config.tasks.clear();
            }

            *run =
                !Gui::task_list_window(&ui, &mut config, &mut self.state, &self.macro_labels[..]);
            Gui::add_tasks_window(&ui, &mut self.state, &self.job_labels[..]);
            Gui::configuration_window(&ui, &mut config);
            // Always try to render a popup in case we have data primed for one.
            Gui::modal_popup_window(&ui, &mut self.state);

            // Modifications to the task list can't be done while we're rendering
            // it, so the actions are deferred to here.
            if let Some(id) = self.state.task_to_remove {
                config.tasks.remove(id as usize);
                self.state.task_to_remove = None;
            }

            if let Some((t_id, offset)) = self.state.task_to_move {
                let pos = (t_id + offset) as usize;
                let task = config.tasks.remove(t_id as usize);
                config.tasks.insert(pos, task);
                self.state.task_to_move = None;
            }

            // If |Add Task| was clicked then we fire off a query to
            // xivapi via the worker thread and add the results to
            // the task list if they were successful.
            if self.state.add_task_button_clicked {
                if !self.state.searching {
                    self.rpc_tx
                        .send(Request::Recipe {
                            item: self.state.search_str.to_string(),
                            job: self.state.search_job as u32,
                        })
                        .unwrap_or_else(|e| {
                            log::error!("rpc failed at line {}: {}", line!(), e.to_string())
                        });
                    self.state.searching = true;
                }

                // If we're in a searching state then keep trying to read a
                // response from the worker while we're rendering the gui.
                if let Ok(r) = self.rpc_rx.try_recv() {
                    if let Response::Recipe(Some(recipe)) = r {
                        config.tasks.push(Task::new(recipe));
                    } else {
                        let msg = &format!(
                            "No {} results found on XIVApi for \"{}\"",
                            xiv::JOBS[self.state.search_job as usize],
                            &self.state.search_str
                        );
                        Gui::set_modal_text(&mut self.state, "Item not found", msg);
                    }
                    self.state.searching = false;
                    self.state.add_task_button_clicked = false;
                }
            }
        });

        Ok(self.state.begin_crafts_selected)
    }

    fn _display_unimplemented(state: &mut UiState) {
        Gui::set_modal_text(
            state,
            "Unimplemented",
            "This feature is not yet implemented, sorry!",
        );
    }

    /// Stores the strings for the modal pop-up and sets it to appear on the next frame.
    fn set_modal_text(state: &mut UiState, title: &str, msg: &str) {
        state.modal_popup.title.clear();
        state.modal_popup.title.push_str(title);
        state.modal_popup.msg.clear();
        state.modal_popup.msg.push_str(msg);
        state.modal_popup.has_msg = true;
    }

    /// Check modal state and draw it if necessary. It will block all user input
    /// from the other windows.
    ///
    /// Much of this function is dealing with the fact that we cannot currently
    /// set the size of a modal window with the Rust imgui bindings.
    fn modal_popup_window(ui: &imgui::Ui, state: &mut UiState) {
        if state.modal_popup.has_msg {
            let title = state.modal_popup.title.clone();
            ui.open_popup(&title);
            ui.popup_modal(&title)
                .always_auto_resize(true)
                .resizable(false)
                .movable(false)
                .build(|| {
                    ui.text(&state.modal_popup.msg);
                    if ui.button(im_str!("Ok"), [0.0, 0.0]) {
                        ui.close_current_popup();
                        state.modal_popup.has_msg = false;
                    }
                });
        }
    }

    fn main_menu(ui: &imgui::Ui, state: &mut UiState) {
        ui.main_menu_bar(|| {
            ui.menu(im_str!("Crafting Tasks")).build(|| {
                ui.menu_item(im_str!("Begin Crafting"))
                    .selected(&mut state.begin_crafts_selected)
                    .build();
                ui.separator();
                ui.menu_item(im_str!("Clear List"))
                    .selected(&mut state.clear_task_list)
                    .build();
            });
        });
    }

    fn progress_window(ui: &imgui::Ui, state: &mut UiState) {
        ui.open_popup(im_str!("Crafting Progress"));
        ui.popup_modal(im_str!("Crafting Progress"))
            .always_auto_resize(true)
            .build(|| {
                // Progress bars look better with borders.
                let _token = ui.push_style_var(StyleVar::FrameBorderSize(1.0));
                for s in state.craft_status.iter() {
                    ui.text(format!("{} {}/{}", s.name, s.finished, s.total));
                    ui.progress_bar(s.finished as f32 / s.total as f32).build();
                }
                if !state.waiting_to_stop {
                    if ui.button(im_str!("Stop"), [0.0, 0.0]) {
                        // Ensure the worker thread stops crafting.
                        state.craft_stopped = true;
                    }
                } else {
                    ui.text("Waiting for item to finish");
                }
            });
    }

    /// The window recipe searching and adding items to the task list.
    fn add_tasks_window(ui: &imgui::Ui, state: &mut UiState, job_labels: &[ImString]) {
        ui.window(im_str!("Add Tasks"))
            .size(ADD_TASKS_SIZE, Condition::FirstUseEver)
            .position(ADD_TASKS_POSITION, Condition::FirstUseEver)
            .collapsible(false)
            .movable(false)
            .resizable(false)
            .build(|| {
                gui_support::combobox(
                    ui,
                    im_str!("Job (if multiple)"),
                    &mut state.search_job,
                    job_labels,
                );
                if ui
                    .input_text(im_str!("Item"), &mut state.search_str)
                    .flags(
                        ImGuiInputTextFlags::EnterReturnsTrue | ImGuiInputTextFlags::AutoSelectAll,
                    )
                    .build()
                    || ui.button(im_str!("Add"), [0.0, 0.0])
                {
                    state.add_task_button_clicked = true;
                }
            });
    }

    /// The task list display itself.
    fn task_list_window(
        ui: &imgui::Ui,
        config: &mut config::Config,
        state: &mut UiState,
        macros: &[ImString],
    ) -> bool {
        ui.window(im_str!("Task List"))
            .size(TASK_LIST_SIZE, Condition::FirstUseEver)
            .position(TASK_LIST_POSITION, Condition::FirstUseEver)
            .collapsible(false)
            .resizable(false)
            .scroll_bar(true)
            .movable(false)
            .build(|| {
                // Both Tasks and their materials are enumerated so we can generate unique
                // UI ids for widgets and prevent any sort of UI clash.
                let task_len = config.tasks.len();
                for (task_id, mut t) in &mut config.tasks.iter_mut().enumerate() {
                    Gui::draw_task(ui, state, task_len as i32, task_id as i32, &mut t, macros);
                }
            });

        // If we return a |true| value the main_loop will know to exit the run loop.
        state.exit_gui
    }

    /// A helper function called to render each task in the task list.
    fn draw_task(
        ui: &imgui::Ui,
        state: &mut UiState,
        task_cnt: i32,
        task_id: i32,
        task: &mut Task,
        macros: &[ImString],
    ) {
        ui.push_id(task_id);
        // header should be closeable
        let header_name = ImString::new(format!(
            "[{} {}] {}x {} {}",
            xiv::JOBS[task.recipe.job as usize],
            task.recipe.level,
            task.quantity,
            task.recipe.name,
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
            ui.indent();
            ui.text(format!(
                "{} Durability  {} Difficulty  {} Quality",
                task.recipe.durability, task.recipe.difficulty, task.recipe.quality
            ));
            ui.checkbox(im_str!("Use any materials."), &mut task.use_any_mats);

            // Draw material widgets, or just the checkbox if checked.
            for (i, (mat, qual)) in task
                .recipe
                .mats
                .iter()
                .zip(task.mat_quality.iter_mut())
                .enumerate()
            {
                ui.push_id(i as i32);
                if !task.use_any_mats {
                    // Otherwise we need to convert some numerical values to strings,
                    // then feed them into the widgets. This seems like it should
                    // thrash like crazy, but thankfully it's 2019 and processors
                    // are fast?
                    let nq_imstr = ImString::new(format!("{} NQ", qual.nq.to_string()));
                    {
                        // width scope
                        let _w = ui.push_item_width(ui.get_window_size()[0] * 0.25);
                        ui.text(&ImString::new(mat.name.clone()));
                        ui.text(nq_imstr);
                        ui.same_line(0.0);
                        // Use a temp to deal with imgui only allowing i32
                        let mut hq: i32 = qual.hq as i32;
                        if ui.input_int(im_str!("HQ"), &mut hq).build() {
                            qual.hq = min(max(0, hq as u32), mat.count);
                            qual.nq = mat.count - qual.hq;
                        }
                    }
                }
                ui.pop_id();
            }
            {
                // width scope
                let _w = ui.push_item_width(ui.get_window_size()[0] * 0.33);
                ui.checkbox(im_str!("Collectable"), &mut task.is_collectable);
                if ui.input_int(im_str!("Count"), &mut task.quantity).build() {
                    task.quantity = max(1, task.quantity);
                }
            }
            gui_support::combobox(ui, im_str!("Macro"), &mut task.macro_id, &macros);
            if task_id > 0 {
                if ui.small_button(im_str!("up")) {
                    state.task_to_move = Some((task_id, -1));
                }
                ui.same_line(0.0);
            }
            if task_id < task_cnt - 1 {
                if ui.small_button(im_str!("down")) {
                    state.task_to_move = Some((task_id, 1));
                }
                ui.same_line(0.0);
            }
            if ui.small_button(im_str!("delete")) {
                state.task_to_remove = Some(task_id);
            }
            ui.unindent();
        }
        ui.pop_id();
    }

    /// The window for all configuration and optional settings.
    fn configuration_window(ui: &imgui::Ui, config: &mut config::Config) {
        ui.window(im_str!("Configuration"))
            .size(CONFIGURATION_SIZE, Condition::FirstUseEver)
            .position(CONFIGURATION_POSITION, Condition::FirstUseEver)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .build(|| {
                if ui
                    .collapsing_header(im_str!("Gear Sets"))
                    .default_open(true)
                    .build()
                {
                    let _w = ui.push_item_width(ui.get_window_size()[0] * 0.33);
                    for (i, name) in xiv::JOBS.iter().enumerate() {
                        if ui
                            .input_int(&ImString::new(*name), &mut config.options.gear[i])
                            .build()
                        {
                            config.options.gear[i] = max(config.options.gear[i], 0);
                        }
                    }
                    if ui
                        .input_int(
                            im_str!("Non-DoH set (for Collectable safety)"),
                            &mut config.options.non_doh_gear,
                        )
                        .build()
                    {
                        config.options.non_doh_gear = max(config.options.non_doh_gear, 0);
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
                    ui.indent();
                    ui.text_wrapped(im_str!(
                        "Tasks will be saved when tasks are started, \
                         or the config is saved."
                    ));
                    ui.unindent();
                    ui.checkbox(
                        im_str!("Use slower menu navigation"),
                        &mut config.options.use_slow_navigation,
                    );
                    ui.indent();
                    ui.text_wrapped(im_str!(
                        "Use this option if you have a lower (<30) fps or higher (200ms+?) \
                         latency."
                    ));
                    ui.unindent();
                };
                if ui.button(im_str!("Save changes"), [0.0, 0.0])
                    && write_config(None, config).is_err()
                {
                    log::error!("Error writing config :(")
                }
            });
    }

    /// Ensures all gear sets are configured for a given list of tasks before
    /// starting crafting.
    fn check_gear_sets(mut state: &mut UiState, config: &config::Config) -> bool {
        for task in &config.tasks {
            let job = task.recipe.job as usize;
            if config.options.gear[job] == 0 {
                log::error!("No gear set configured for {}", xiv::JOBS[job]);
                let msg = format!("Please set a gear set for {} to continue", xiv::JOBS[job]);
                Gui::set_modal_text(&mut state, "Unconfigured gear sets", &msg);
                return false;
            }
        }
        true
    }
}
