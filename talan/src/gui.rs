use crate::config::{self, write_config};
use crate::macros::MacroFile;
use crate::rpc::{Request, Response};
use crate::task::{Status, Task};
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

#[derive(Debug, PartialEq)]
enum WorkerStatus {
    Idle,
    Crafting,
    Stopping,
}

/// UiState tracks all the state specific to ImGui and any widget
/// events triggered by the user. All widgets that bind to data
/// will find their backing store here.
#[derive(Debug)]
struct UiState {
    worker: WorkerStatus,
    craft_status: Option<Vec<Status>>,
    /// Store for the Error / Message popup
    modal_popup: ModalText,
    // The item search string.
    search_str: ImString,
    // The job dropdown selection.
    search_job: i32,
}

impl Default for UiState {
    fn default() -> UiState {
        UiState {
            modal_popup: ModalText {
                has_msg: false,
                title: ImString::with_capacity(128),
                msg: ImString::with_capacity(512),
            },
            worker: WorkerStatus::Idle,
            craft_status: None,
            search_str: ImString::with_capacity(128),
            search_job: 0,
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

    fn send_to_worker(&self, r: Request) {
        self.rpc_tx
            .send(r)
            .unwrap_or_else(|e| log::error!("rpc failed at line {}: {}", line!(), e.to_string()))
    }

    pub fn start(&mut self, mut config: &mut config::Config) {
        let system = gui_support::init(f64::from(WINDOW_W), f64::from(WINDOW_H), "Talan");

        system.main_loop(|_, ui| {
            // Most operations (recipe queries, crafting, etc) are handled by
            // the background worker thread. This means we can always update bookkeeping
            // and other state by checking if there are any messages on the channel.
            if let Ok(resp) = self.rpc_rx.try_recv() {
                match resp {
                    Response::Recipe(recipe_opt) => {
                        if let Some(recipe) = recipe_opt {
                            config.tasks.push(Task::new(recipe));
                        } else {
                            let msg = &format!(
                                "No {} results found on XIVApi for \"{}\"",
                                xiv::JOBS[self.state.search_job as usize],
                                &self.state.search_str
                            );
                            Gui::set_modal_text(&mut self.state, "Item not found", msg);
                        }
                    }
                    Response::Craft(status) => {
                        // There is a final status sent when the worker is told to stop,
                        // before the EOW. This lets us track the final item completion
                        // and reflect it in the progress bars, but we need to stay out
                        // of the crafting state to hide the |Stop| button.
                        if self.state.worker != WorkerStatus::Stopping {
                            self.state.worker = WorkerStatus::Crafting;
                        }
                        self.state.craft_status = Some(status);
                    }
                    Response::EOW => {
                        self.state.worker = WorkerStatus::Idle;
                        self.state.craft_status = None;
                    }
                }
            }

            // Everything is rendered unconditionally here because the methods
            // themselves are data driven based on the state structure.
            self.main_menu(&ui, &mut config);
            self.task_list_window(&ui, &mut config);
            self.add_tasks_window(&ui);
            Gui::configuration_window(&ui, &mut config);
            // Always try to render a popup in case we have data primed for one.
            self.modal_popup_window(&ui);
            self.progress_window(&ui);
        });
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
    fn modal_popup_window(&mut self, ui: &imgui::Ui) {
        if self.state.modal_popup.has_msg {
            let title = self.state.modal_popup.title.clone();
            ui.open_popup(&title);
            ui.popup_modal(&title)
                .always_auto_resize(true)
                .resizable(false)
                .movable(false)
                .build(|| {
                    ui.text(&self.state.modal_popup.msg);
                    if ui.button(im_str!("Ok"), [0.0, 0.0]) {
                        ui.close_current_popup();
                        self.state.modal_popup.has_msg = false;
                    }
                });
        }
    }

    fn main_menu(&mut self, ui: &imgui::Ui, config: &mut config::Config) {
        ui.main_menu_bar(|| {
            ui.menu(im_str!("Crafting Tasks")).build(|| {
                if ui.menu_item(im_str!("Begin Crafting")).build()
                    && Gui::check_gear_sets(&mut self.state, config)
                {
                    self.send_to_worker(Request::Craft {
                        options: config.options,
                        tasks: config.tasks.clone(),
                        macros: self.macros.to_vec(),
                    });
                }

                ui.separator();
                if ui.menu_item(im_str!("Clear List")).build() {
                    config.tasks.clear();
                }
            });
        });
    }

    fn progress_window(&mut self, ui: &imgui::Ui) {
        if self.state.craft_status.is_some() {
            ui.open_popup(im_str!("Crafting Progress"));
            ui.popup_modal(im_str!("Crafting Progress"))
                .always_auto_resize(true)
                .build(|| {
                    if let Some(status) = &self.state.craft_status {
                        // Progress bars look better with borders.
                        let _token = ui.push_style_var(StyleVar::FrameBorderSize(1.0));
                        for s in status.iter() {
                            ui.text(format!("{} {}/{}", s.name, s.finished, s.total));
                            ui.progress_bar(s.finished as f32 / s.total as f32).build();
                        }
                        if self.state.worker == WorkerStatus::Crafting {
                            if ui.button(im_str!("Stop"), [0.0, 0.0]) {
                                self.send_to_worker(Request::StopCrafting);
                                // Ensure the worker thread stops crafting.
                                self.state.worker = WorkerStatus::Stopping;
                            }
                        } else {
                            ui.text("Waiting for any queued actions to finish");
                        }
                    }
                });
        }
    }

    /// The window recipe searching and adding items to the task list.
    fn add_tasks_window(&mut self, ui: &imgui::Ui) {
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
                    &mut self.state.search_job,
                    &self.job_labels,
                );
                if ui
                    .input_text(im_str!("Item"), &mut self.state.search_str)
                    .flags(
                        ImGuiInputTextFlags::EnterReturnsTrue | ImGuiInputTextFlags::AutoSelectAll,
                    )
                    .build()
                    || ui.button(im_str!("Add"), [0.0, 0.0])
                {
                    self.send_to_worker(Request::Recipe {
                        item: self.state.search_str.to_string(),
                        job: self.state.search_job as u32,
                    });
                }
            });
    }

    /// The task list display itself.
    fn task_list_window(&mut self, ui: &imgui::Ui, config: &mut config::Config) {
        let mut tasks_copy = config.tasks.clone();
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
                for (task_id, mut t) in &mut config.tasks.iter_mut().enumerate() {
                    self.draw_task(ui, task_id, &mut t, &mut tasks_copy);
                }
            });
        config.tasks = tasks_copy;
    }

    /// A helper function called to render each task in the task list.
    fn draw_task(
        &mut self,
        ui: &imgui::Ui,
        task_id: usize,
        task: &mut Task,
        tasks_copy: &mut Vec<Task>,
    ) {
        ui.push_id(task_id as i32);
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
                    // We need to convert some numerical values to strings,
                    // then feed them into the widgets. This seems like it should
                    // thrash like crazy, but thankfully it's 2019 and processors
                    // are fast? This is a side effect of not wanting NQ to have
                    // buttons that the integer widget has, and not wanting the
                    // unaligned text of a label.
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
            gui_support::combobox(ui, im_str!("Macro"), &mut task.macro_id, &self.macro_labels);
            // None of these task modifications can happen at the same time becaise it's
            // not possible for a user to click multiple buttons in the same frame.
            if task_id > 0 {
                if ui.small_button(im_str!("up")) {
                    let t = tasks_copy.remove(task_id);
                    tasks_copy.insert(task_id - 1, t);
                }
                ui.same_line(0.0);
            }
            if task_id < tasks_copy.len() - 1 {
                if ui.small_button(im_str!("down")) {
                    let t = tasks_copy.remove(task_id);
                    tasks_copy.insert(task_id + 1, t);
                }
                ui.same_line(0.0);
            }
            if ui.small_button(im_str!("delete")) {
                tasks_copy.remove(task_id);
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
    fn check_gear_sets(state: &mut UiState, config: &config::Config) -> bool {
        for task in &config.tasks {
            let job = task.recipe.job as usize;
            if config.options.gear[job] == 0 {
                log::error!("No gear set configured for {}", xiv::JOBS[job]);
                let msg = format!("Please set a gear set for {} to continue", xiv::JOBS[job]);
                Gui::set_modal_text(state, "Unconfigured gear sets", &msg);
                return false;
            }
        }
        true
    }
}
