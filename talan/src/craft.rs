//use crate::role_actions::RoleActions;
use crate::config::Config;
use crate::macros::{Action, MacroFile};
use crate::task::Task;
use log;
use std::thread::sleep;
use std::time::{Duration, Instant};
use xiv::ui;

// Milliseconds to pad the GCD to account for latency
const GCD_PADDING: u64 = 200;

// Runs through the set of tasks
pub fn craft_items(handle: xiv::XivHandle, cfg: &Config, macros: &[MacroFile]) {
    // Get the UI into a state we can trust it, and pray the user doesn't touch it.
    ui::clear_window(handle);

    // Clear role actions before we iterate tasks so the game state
    // and role action state will be in sync.
    let mut job: u32 = 256;
    let mut _gearset: u64 = 0;
    let mut _first_task: bool = true;

    if cfg.non_doh_gear != 0 {
        change_gearset(handle, cfg.non_doh_gear);
        ui::wait(1.0);
    }

    for task in &cfg.tasks {
        log::trace!("Task: {:?}", task);
        let task_job: usize = task.recipe.job as usize;

        if cfg.gear[task_job] == 0 {
            panic!(
                "No gear set is configured for {}, aborting tasks!",
                xiv::JOBS[task_job]
            );
        }

        // Swap our job if necessary. It may have been used in the previous task.
        if job != task.recipe.job {
            log::trace!("changing job to {}.", xiv::JOBS[task_job]);
            change_gearset(handle, cfg.gear[task_job]);
            job = task.recipe.job
        } else {
            log::trace!("already {}, no need to change job.", xiv::JOBS[task_job]);
        }

        if task.is_collectable {
            toggle_collectable(handle);
        }

        // Navigate to the correct recipe based on the index provided
        select_recipe(handle, &task);

        // Time to craft the items
        execute_task(handle, &task, &macros[task.macro_id as usize].actions[..]);

        ui::press_escape(handle);
        ui::wait(2.0);

        if task.is_collectable {
            toggle_collectable(handle);
        }
    }
}

pub fn open_craft_window(handle: xiv::XivHandle) {
    ui::send_key(handle, 'N' as i32);
    ui::wait(1.0);
}

// Selects the appropriate recipe then leaves the cursor on the Synthesize
// button, ready for material selection.
pub fn select_recipe(handle: xiv::XivHandle, task: &Task) {
    // Bring up the crafting window itself and give it time to appear
    open_craft_window(handle);
    log::info!("selecting recipe...");
    // Loop backward through the UI 9 times to ensure we hit the text box
    // no matter what crafting class we are. The text input boxes are strangely
    // modal so that if we select them at any point they will hold on to focus
    // for characters.
    //
    // Move left
    // The crafting window always starts with the current job selected and if we press
    // |BACK| 1 more time than the job's index then we will end up at the search box.
    for _ in 0..=task.recipe.job + 1 {
        ui::cursor_backward(handle);
    }
    ui::press_confirm(handle);
    ui::wait(1.0);
    ui::send_string(handle, &task.recipe.name);
    ui::press_enter(handle);
    ui::wait(1.0);
    // Navigate to the offset we need
    for _ in 0..task.recipe.index {
        ui::cursor_down(handle);
    }

    // Select the recipe to get to components / sythen
    ui::press_confirm(handle);
}

pub fn select_materials(handle: xiv::XivHandle, task: &Task) {
    log::info!("selecting materials...");
    ui::cursor_up(handle);
    // TODO implement HQ > NQ
    ui::cursor_right(handle);
    ui::cursor_right(handle);

    // The cursor should be on the quantity field of the bottom item now
    // We move through the ingredients backwards because we start at the bottom of t
    for (i, material) in task.recipe.mats.iter().enumerate().rev() {
        log::trace!("{}x {}", material.count, material.name);
        for _ in 0..material.count {
            ui::press_confirm(handle)
        }
        // Don't move up if we've made it back to the top of the ingredients
        if i != 0 {
            ui::cursor_up(handle);
        }
    }
    ui::cursor_left(handle);
    for material in &task.recipe.mats {
        for _ in 0..material.count {
            ui::press_confirm(handle)
        }
        ui::cursor_down(handle);
    }
}

fn execute_task(handle: xiv::XivHandle, task: &Task, actions: &[Action]) {
    for task_index in 1..=task.quantity {
        log::info!(
            "crafting {} {}/{}",
            task.recipe.name,
            task_index,
            task.quantity
        );
        // If we're at the start of a task we will already have the Synthesize button
        // selected with the pointer.
        select_materials(handle, &task);
        ui::press_confirm(handle);

        // The first action is one second off so we start typing while the
        // crafting window is coming up.
        let mut next_action = Instant::now() + Duration::from_secs(1);
        let mut prev_action = next_action;
        for action in actions {
            ui::press_enter(handle);
            ui::send_string(handle, &format!("/ac \"{}\"", &action.name));
            // At this point the action is queued in the text buffer, so we can
            // wait the GCD duration based on the last action we sent.
            let mut now = Instant::now();
            if now < next_action {
                let delta = next_action - now;
                log::trace!("sleeping {:?}", delta);
                sleep(delta);
            }
            ui::press_enter(handle);
            // Handle turning a 3.0 wait in traditional macros into something closer to a real GCD
            let delay = if action.wait == 2 { 1500 } else { 2500 };

            now = Instant::now();
            log::debug!("action: {} ({:?})", action.name, now - prev_action);
            prev_action = now;
            next_action = now + Duration::from_millis(delay + GCD_PADDING);
        }

        // Wait for the last GCD to finish
        sleep(next_action - Instant::now());

        // There are two paths here. If an item is collectable then it will
        // prompt a dialog to collect the item as collectable. In this case,
        // selecting confirm with the keyboard will bring the cursor up already.
        // The end result is that it needs fewer presses of the confirm key
        // than otherwise.
        //
        // At the end of this sequence the cursor should have selected the recipe
        // again and be on the Synthesize button.
        if task.is_collectable {
            ui::wait(1.0);
            ui::press_confirm(handle);
            // Give the UI a moment
            ui::wait(3.0);
            ui::press_confirm(handle)
        } else {
            ui::wait(3.0);
            ui::press_confirm(handle);
        }
    }
}

fn send_action(handle: xiv::XivHandle, action: &str) {
    ui::press_enter(handle);
    ui::send_string(handle, &format!("/ac \"{}\"", action));
    ui::press_enter(handle);
}

pub fn change_gearset(handle: xiv::XivHandle, gearset: i32) {
    log::info!("changing to gearset {}", gearset);
    ui::press_enter(handle);
    ui::send_string(handle, &format!("/gearset change {}", gearset));
    ui::press_enter(handle);
}

pub fn toggle_collectable(handle: xiv::XivHandle) {
    send_action(handle, &"collectable synthesis");
}
