use crate::config::{self, write_config};
use crate::lists::import_tasks_from_clipboard;
use crate::macros::{get_macro_for_recipe, read_macros_from_file, Macro};
use crate::rpc::{Request, Response};
use crate::task::{Status, Task};
use std::path::PathBuf;

use gui_support;
use imgui::*;
use std::cmp::{max, min};
use std::sync::mpsc::{Receiver, Sender};

// These represent a ratio compared to WINDOW_W and WINDOW_H
const CONFIGURATION_SIZE: [f32; 2] = [300.0, 0.0];

const WINDOW_SIZE: [f32; 2] = [1024.0, 768.0];

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

#[derive(Copy, Clone, Debug)]
enum TaskListModification {
    MoveDown(usize),
    MoveUp(usize),
    Delete(usize),
}

/// UiState tracks all the state specific to ImGui and any widget
/// events triggered by the user. All widgets that bind to data
/// will find their backing store here.
#[derive(Debug)]
struct UiState {
    previous_window_size: [f32; 2],
    worker: WorkerStatus,
    craft_status: Option<Vec<Status>>,
    /// Store for the Error / Message popup
    modal_popup: ModalText,
    // The item search string.
    search_str: ImString,
    // The job dropdown selection.
    macros: Vec<Macro>,
    search_job: usize,
    show_gear_set_window: bool,
    task_list_modification: Option<TaskListModification>,
    should_exit: bool,
}

impl Default for UiState {
    fn default() -> UiState {
        UiState {
            previous_window_size: [0.0, 0.0],
            modal_popup: ModalText {
                has_msg: false,
                title: ImString::with_capacity(128),
                msg: ImString::with_capacity(512),
            },
            worker: WorkerStatus::Idle,
            macros: Vec::new(),
            craft_status: None,
            search_str: ImString::with_capacity(128),
            search_job: 0,
            show_gear_set_window: false,
            task_list_modification: None,
            should_exit: false,
        }
    }
}

pub struct Gui<'a> {
    config_path: PathBuf,
    macro_path: PathBuf,
    state: UiState,
    job_labels: Vec<ImString>,
    rpc_tx: &'a Sender<Request>,
    rpc_rx: &'a Receiver<Response>,
}

impl<'a, 'b> Gui<'a> {
    pub fn new(
        config_path: PathBuf,
        macro_path: PathBuf,
        rpc_tx: &'a Sender<Request>,
        rpc_rx: &'a Receiver<Response>,
    ) -> Gui<'a> {
        Gui {
            config_path,
            macro_path,
            state: UiState::default(),
            job_labels: xiv::JOBS
                .iter()
                .map(|&j| ImString::new(j.to_owned()))
                .collect(),
            rpc_tx,
            rpc_rx,
        }
    }

    fn send_to_worker(&self, r: Request) {
        self.rpc_tx
            .send(r)
            .unwrap_or_else(|e| log::error!("rpc failed at line {}: {}", line!(), e.to_string()))
    }

    pub fn start(&mut self, mut config: &mut config::Config) {
        let system = gui_support::init(
            f64::from(WINDOW_SIZE[0]),
            f64::from(WINDOW_SIZE[1]),
            "Talan",
        );

        // Load the macros and remap any tasks that need it.
        match read_macros_from_file(&self.macro_path, &mut self.state.macros) {
            Ok(()) => {
                // Load saved tasks and re-map the macros in case the macro file changed.
                for task in &mut config.tasks {
                    task.macro_id = get_macro_for_recipe(
                        &self.state.macros,
                        &task.recipe,
                        config.options.specialist[task.recipe.job as usize],
                    );
                }
            }
            Err(e) => log::error!("Failed to read macros: {}", e),
        }

        system.main_loop(|_, ui| {
            if self.state.should_exit {
                return;
            }

            // Most operations (recipe queries, crafting, etc) are handled by
            // the background worker thread. This means we can always update bookkeeping
            // and other state by checking if there are any messages on the channel.
            if let Ok(resp) = self.rpc_rx.try_recv() {
                match resp {
                    Response::Recipe { recipe, count } => {
                        if let Some(r) = recipe {
                            let craft_cnt = (count as f32 / r.result_amount as f32).ceil() as u32;
                            let mut task = Task::new(r, craft_cnt);
                            task.macro_id = get_macro_for_recipe(
                                &self.state.macros,
                                &task.recipe,
                                config.options.specialist[task.recipe.job as usize],
                            );
                            config.tasks.push(task);
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

            // If the user wanted to move task position it's handled here before the next render.
            if let Some(task_mod) = self.state.task_list_modification {
                match task_mod {
                    TaskListModification::MoveUp(idx) => {
                        let task = config.tasks.remove(idx);
                        config.tasks.insert(idx - 1, task);
                    }
                    TaskListModification::MoveDown(idx) => {
                        let task = config.tasks.remove(idx);
                        config.tasks.insert(idx + 1, task);
                    }
                    TaskListModification::Delete(idx) => {
                        config.tasks.remove(idx);
                    }
                };
            }
            self.state.task_list_modification = None;

            // Everything is rendered unconditionally here because the methods
            // themselves are data driven based on the state structure.
            self.main_menu(&ui, &mut config);
            self.add_tasks_window(&ui);
            self.task_list_window(&ui, &mut config);
            if self.state.show_gear_set_window {
                self.gear_set_window(&ui, &mut config);
            }
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
        if let Some(main_menu) = ui.begin_main_menu_bar() {
            self.state.previous_window_size = ui.window_size();
            if let Some(menu) = ui.begin_menu(im_str!("File"), true) {
                if MenuItem::new(im_str!("Save All"))
                    .shortcut(im_str!("Ctrl+S"))
                    .build(&ui)
                {
                    match write_config(Some(&self.config_path), config) {
                        Ok(_) => log::info!("Wrote configuration to disk."),
                        Err(e) => log::error!("Failed to write configuration: {}", e.to_string()),
                    };
                }
                ui.separator();
                MenuItem::new(im_str!("Exit"))
                    .shortcut(im_str!("Ctrl+Q"))
                    .build_with_ref(&ui, &mut self.state.should_exit);
                menu.end(ui);
            }
            if let Some(menu) = ui.begin_menu(im_str!("Tasks"), true) {
                if MenuItem::new(im_str!("Craft All"))
                    .shortcut(im_str!("F5"))
                    .build(&ui)
                {
                    // Get clippy to leave us alone about collapsing the if
                    if Gui::check_gear_sets(&mut self.state, config) {
                        self.send_to_worker(Request::Craft {
                            options: config.options,
                            tasks: config.tasks.clone(),
                            macros: self.state.macros.clone(),
                        });
                    }
                }
                if MenuItem::new(im_str!("Import From Clipboard"))
                    .shortcut(im_str!("Ctrl+I"))
                    .build(&ui)
                {
                    if let Ok(items) = import_tasks_from_clipboard() {
                        for i in &items {
                            log::debug!("item: {:#?}", i);
                            self.send_to_worker(Request::Recipe {
                                item: i.item.clone(),
                                job: None,
                                count: i.count,
                            });
                        }
                    }
                }
                ui.separator();
                if MenuItem::new(im_str!("Clear Tasks"))
                    .shortcut(im_str!("Ctrl+W"))
                    .build(&ui)
                {
                    config.tasks.clear();
                }
                menu.end(ui);
            }
            if let Some(menu) = ui.begin_menu(im_str!("Options"), true) {
                MenuItem::new(im_str!("Gear Configuration"))
                    .build_with_ref(ui, &mut self.state.show_gear_set_window);
                ui.separator();
                // For MenuItems if we use |build_with_ref| we can't have the menu ite,
                // clear its own state.
                if MenuItem::new(im_str!("Clear XIV Window Before Crafting"))
                    .selected(config.options.should_clear_window_on_craft)
                    .build(ui)
                {
                    config.options.should_clear_window_on_craft =
                        !config.options.should_clear_window_on_craft;
                }
                if MenuItem::new(im_str!("Use Slow Dialog Navigation"))
                    .selected(config.options.use_slow_dialog_navigation)
                    .build(ui)
                {
                    config.options.use_slow_dialog_navigation =
                        !config.options.use_slow_dialog_navigation;
                }
                if MenuItem::new(im_str!("Change Job Before Starting Tasks"))
                    .selected(config.options.swap_job_before_tasks)
                    .build(ui)
                {
                    config.options.swap_job_before_tasks = !config.options.swap_job_before_tasks;
                }
                menu.end(ui);
            }
            main_menu.end(ui);
        }
    }

    fn progress_window(&mut self, ui: &imgui::Ui) {
        if self.state.craft_status.is_some() {
            ui.open_popup(im_str!("Crafting Progress"));
            ui.popup_modal(im_str!("Crafting Progress"))
                .resizable(true)
                .always_auto_resize(true)
                .build(|| {
                    if let Some(status) = &self.state.craft_status {
                        // Pad out the width of the box and then reset the cursor so there's no floating widget behavior.
                        let pos = ui.cursor_pos();
                        ui.text(" ".repeat(100));
                        ui.set_cursor_pos(pos);
                        // Progress bars look better with borders.
                        let token = ui.push_style_var(StyleVar::FrameBorderSize(1.0));
                        for s in status.iter() {
                            let label =
                                &ImString::new(format!("{} {}/{}", s.name, s.finished, s.total));
                            ProgressBar::new(s.finished as f32 / s.total as f32)
                                .overlay_text(label)
                                .build(&ui);
                        }
                        ui.text(" ".repeat(100));
                        if self.state.worker == WorkerStatus::Crafting {
                            if ui.button(im_str!("Stop"), [0.0, 0.0]) {
                                self.send_to_worker(Request::StopCrafting);
                                // Ensure the worker thread stops crafting.
                                self.state.worker = WorkerStatus::Stopping;
                            }
                        } else {
                            ui.text("Waiting for any queued actions to finish");
                        }
                        token.pop(&ui);
                    }
                });
        }
    }

    /// The window recipe searching and adding items to the task list.
    fn add_tasks_window(&mut self, ui: &imgui::Ui) {
        Window::new(im_str!("Recipe Search"))
            .size([self.state.previous_window_size[0], 0.0], Condition::Always)
            .position([0.0, self.state.previous_window_size[1]], Condition::Always)
            .always_auto_resize(true)
            .collapsible(false)
            .movable(false)
            .resizable(false)
            .build(&ui, || {
                self.state.previous_window_size[1] += ui.window_size()[1];
                let labels: Vec<&ImStr> = self
                    .job_labels
                    .iter()
                    .map(std::convert::AsRef::as_ref)
                    .collect();
                {
                    let _w = ui.push_item_width(100.0);
                    ComboBox::new(im_str!("Job")).build_simple_string(
                        ui,
                        &mut self.state.search_job,
                        &labels,
                    );
                }
                ui.same_line(0.0);
                let mut perform_search = false;
                {
                    let _ = ui.push_item_width(300.0);
                    if ui
                        .input_text(im_str!("Item"), &mut self.state.search_str)
                        .flags(
                            ImGuiInputTextFlags::EnterReturnsTrue
                                | ImGuiInputTextFlags::AutoSelectAll,
                        )
                        .build()
                    {
                        perform_search = true;
                    }
                }
                ui.same_line(0.0);
                if ui.button(im_str!("Add"), [0.0, 0.0]) {
                    perform_search = true;
                }

                if perform_search {
                    self.send_to_worker(Request::Recipe {
                        item: self.state.search_str.to_string(),
                        job: Some(self.state.search_job as u32),
                        count: 1,
                    });
                }
            });
    }

    /// The task list display itself.
    fn task_list_window(&mut self, ui: &imgui::Ui, config: &mut config::Config) {
        Window::new(im_str!("Task List"))
            .size(
                [
                    self.state.previous_window_size[0],
                    WINDOW_SIZE[1] - self.state.previous_window_size[1],
                ],
                Condition::Always,
            )
            .position([0.0, self.state.previous_window_size[1]], Condition::Always)
            .resizable(false)
            .collapsible(false)
            .scroll_bar(true)
            .movable(false)
            .build(&ui, || {
                // Both Tasks and their materials are enumerated so we can generate unique
                // UI ids for widgets and prevent any sort of UI clash.
                let task_count = config.tasks.len();
                for (task_id, mut task) in &mut config.tasks.iter_mut().enumerate() {
                    let id = ui.push_id(task_id as i32);
                    let header_name = ImString::new(format!(
                        "[{}] {}x {} {} (recipe lvl {} | {} durability | {} difficulty | {} quality)",
                        xiv::JOBS[task.recipe.job as usize],
                        task.quantity * task.recipe.result_amount,
                        task.recipe.name,
                        if task.is_collectable {
                            "(Collectable)"
                        } else {
                            ""
                        },
                        task.recipe.level,
                        task.recipe.durability,
                        task.recipe.difficulty,
                        task.recipe.quality
                    ));
                    if ui
                        .collapsing_header(&header_name)
                        .default_open(true)
                        .build()
                    {
                        // For the layout of:
                        // | Count |      Macro     |  Collectable  | Specify materials
                        ui.columns(4, im_str!("## Recipe Columns"), false /* no border */);
                        ui.set_column_width(0, ui.window_size()[0] * 0.2);
                        ui.set_column_width(1, ui.window_size()[0] * 0.4);
                        ui.set_column_width(2, ui.window_size()[0] * 0.2);
                        ui.set_column_width(3, ui.window_size()[0] * 0.2);

                        let mut q: i32 = task.quantity as i32;
                        if ui.input_int(im_str!("#"), &mut q).build() {
                            task.quantity = max(1, q as u32);
                        }
                        ui.next_column();
                        let m_labels: Vec<&ImStr> =
                            self.state.macros.iter().map(|m| m.gui_name.as_ref()).collect();
                        ComboBox::new(im_str!("Macro")).build_simple_string(
                            ui,
                            &mut task.macro_id,
                            &m_labels,
                        );
                        ui.next_column();
                        ui.checkbox(im_str!("Collectable"), &mut task.is_collectable);
                        ui.next_column();
                        ui.checkbox(im_str!("Specify Materials"), &mut task.specify_materials);
                        ui.next_column();

                        // Draw material widgets, or just the checkbox if checked.
                        if task.specify_materials {
                            for (i, (mat, qual)) in task
                                .recipe
                                .mats
                                .iter()
                                .zip(task.mat_quality.iter_mut())
                                .enumerate()
                            {
                                let id = ui.push_id(i as i32);
                                // We need to convert some numerical values to strings,
                                // then feed them into the widgets. This seems like it should
                                // thrash like crazy, but thankfully it's 2019 and processors
                                // are fast? This is a side effect of not wanting NQ to have
                                // buttons that the integer widget has, and not wanting the
                                // unaligned text of a label.
                                ui.next_column(); // Use second column
                                ui.text(&ImString::new(mat.name.clone()));
                                ui.next_column();
                                let nq_imstr = ImString::new(format!("{} NQ", qual.nq.to_string()));
                                ui.text(nq_imstr);
                                ui.next_column();

                                let mut hq: i32 = qual.hq as i32;
                                if ui.input_int(im_str!("HQ"), &mut hq).build() {
                                    qual.hq = min(max(0, hq as u32), mat.count);
                                    qual.nq = mat.count - qual.hq;
                                }
                                ui.next_column();

                                id.pop(&ui);
                            }
                        }

                        // Reset columns
                        ui.columns(1, im_str!("##"), false /* no border */);

                        // None of these task modifications can happen at the same time becaise it's
                        // not possible for a user to click multiple buttons in the same frame.
                        if task_id > 0 {
                            if ui.small_button(im_str!("up")) {
                                self.state.task_list_modification =
                                    Some(TaskListModification::MoveUp(task_id));
                            }
                            ui.same_line(0.0);
                        }
                        if task_id < task_count - 1 {
                            if ui.small_button(im_str!("down")) {
                                self.state.task_list_modification =
                                    Some(TaskListModification::MoveDown(task_id));
                            }
                            ui.same_line(0.0);
                        }
                        if ui.small_button(im_str!("delete")) {
                            self.state.task_list_modification =
                                Some(TaskListModification::Delete(task_id));
                        }
                    }
                    id.pop(&ui);
                }
            });
    }

    /// The window for all configuration and optional settings.
    fn gear_set_window(&mut self, ui: &imgui::Ui, config: &mut config::Config) {
        Window::new(im_str!("Gear Set Configuration"))
            .size(CONFIGURATION_SIZE, Condition::FirstUseEver)
            .opened(&mut self.state.show_gear_set_window)
            .resizable(false)
            .collapsible(false)
            .focused(true)
            .build(&ui, || {
                ui.columns(2, im_str!("gear columns"), false);
                let _w = ui.push_item_width(ui.window_size()[0] * 0.33);
                for (i, name) in xiv::JOBS.iter().enumerate() {
                    if ui
                        .input_int(&ImString::new(*name), &mut config.options.gear[i])
                        .build()
                    {
                        config.options.gear[i] = max(config.options.gear[i], 0);
                    }
                    ui.next_column();
                    let id = ui.push_id(i as i32); // specialists need a unique id
                    if ui.checkbox(im_str!("specialist"), &mut config.options.specialist[i])
                        && config
                            .options
                            .specialist
                            .iter()
                            .fold(0, |acc, &x| acc + (x as i32))
                            > 3
                    {
                        log::error!(
                            "Cannot set {} as a specialist, limit of 3 already reached!",
                            xiv::JOBS[i]
                        );
                        config.options.specialist[i] = false;
                    }
                    ui.next_column();
                    id.pop(&ui);
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
