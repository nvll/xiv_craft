use crate::macros;
use crate::role_actions::RoleActions;
use crate::task::Task;
use crate::ui;
use log;

// Runs through the set of tasks
// TODO: make it actually run more than one task
pub fn craft_items(window: ui::WinHandle, tasks: &[Task]) {
    // TODO: this will be a problem when we run multiple tasks
    // TODO: Investigate why there's always a longer delay after Careful Synthesis II
    // TODO: Tea is going to be a problem for non-specialty recipes
    let mut role_actions = RoleActions::new(window);
    // Clear role actions before we iterate tasks so the game state
    // and role action state will be in sync.
    aaction_clear(window);
    let mut gearset: u64 = 0;
    for task in tasks {
        // Change to the appropriate job if one is set. XIV
        // gearsets start at 1, so 0 is a safe empty value.
        if task.gearset > 0 && task.gearset != gearset {
            // If we're changing jobs we need to set role actions up again,
            // otherwise there's a good chance we can reuse some of the role
            // actions we already have for the next craft
            aaction_clear(window);
            ui::wait_ms(200);
            change_gearset(window, task.gearset);
            gearset = task.gearset;
        }

        clear_windows(window);
        if task.collectable {
            toggle_collectable(window);
        }

        // Check the role action cache and configure any we need for this task
        configure_role_actions(&mut role_actions, task);

        // Bring up the crafting window itself and give it time to appear
        ui::open_craft_window(window);
        ui::wait_secs(1);

        // Navigate to the correct recipe based on the index provided
        select_recipe(window, &task);
        // Time to craft the items
        execute_task(window, &task);

        // Close out of the crafting window and stand up
        clear_windows(window);
        ui::wait_secs(2);
        if task.collectable {
            toggle_collectable(window);
        }
    }
}

fn clear_windows(window: ui::WinHandle) {
    println!("clearing window...");
    // Hitting escape closes one window each. 10 is excessive, but conservative
    for _ in 0..2 {
        ui::escape(window);
    }

    // Cancelling twice will close the System menu if it is open
    ui::cancel(window);
    ui::cancel(window);
    ui::wait_secs(1);
    ui::enter(window);
    ui::enter(window);
}

fn configure_role_actions(role_actions: &mut RoleActions, task: &Task) {
    for action in &task.actions {
        if role_actions.is_role_action(&action.name) {
            role_actions.add_action(&action.name);
            ui::wait_ms(250); // In testing, the game takes 1 second per role action
        }
    }
}

// Selects the appropriate recipe then leaves the cursor on the Synthesize
// button, ready for material selection.
fn select_recipe(window: ui::WinHandle, task: &Task) {
    log::info!("selecting recipe...");
    // Loop backward through the UI 9 times to ensure we hit the text box
    // no matter what crafting class we are. The text input boxes are strangely
    // modal so that if we select them at any point they will hold on to focus
    // for characters.
    //
    // TODO: Link recipe job to this so we don't move more than we need
    for _ in 0..9 {
        ui::move_backward(window);
    }

    ui::confirm(window);
    send_string(window, &task.item.name);
    ui::wait_ms(200);
    ui::enter(window);

    // It takes up to a second for results to populate
    ui::wait_secs(1);

    // Navigate to the offset we need
    for _ in 0..task.index {
        ui::cursor_down(window);
    }

    // Select the recipe to get to components / sythen
    ui::confirm(window);
}

fn select_materials(window: ui::WinHandle, task: &Task) {
    log::info!("selecting materials...");
    ui::cursor_up(window);
    // TODO implement HQ > NQ
    ui::cursor_right(window);
    ui::cursor_right(window);

    // The cursor should be on the quantity field of the bottom item now
    // We move through the ingredients backwards because we start at the bottom of t
    for (i, material) in task.item.materials.iter().enumerate().rev() {
        log::trace!("{}x {}", material.count, material.name);
        for _ in 0..material.count {
            ui::confirm(window)
        }
        // Don't move up if we've made it back to the top of the ingredients
        if i != 0 {
            ui::cursor_up(window);
        }
    }
    ui::cursor_left(window);
    for material in &task.item.materials {
        for _ in 0..material.count {
            ui::confirm(window)
        }
        ui::cursor_down(window);
    }
}

fn execute_task(window: ui::WinHandle, task: &Task) {
    for task_index in 1..=task.count {
        println!("crafting {} {}/{}", task.item.name, task_index, task.count);
        // If we're at the start of a task we will already have the Synthesize button
        // selected with the pointer.
        select_materials(window, &task);
        ui::confirm(window);
        // Wait for the craft dialog to pop up
        ui::wait_secs(2);
        // and now execute the actions
        execute_actions(window, &task.actions);

        // There are two paths here. If an item is collectable then it will
        // prompt a dialog to collect the item as collectable. In this case,
        // selecting confirm with the keyboard will bring the cursor up already.
        // The end result is that it needs fewer presses of the confirm key
        // than otherwise.
        //
        // At the end of this sequence the cursor should have selected the recipe
        // again and be on the Synthesize button.
        if task.collectable {
            ui::wait_secs(1);
            ui::confirm(window);
            // Give the UI a moment
            ui::wait_secs(3);
            ui::confirm(window)
        } else {
            ui::wait_secs(4);
            ui::confirm(window);
        }
    }
}

fn execute_actions(window: ui::WinHandle, actions: &[macros::Action]) {
    for action in actions {
        // Each character has a 20ms wait and the shortest action string
        // we can make (observe or reclaim) is 240 ms, along with 50ms
        // from send_action. That reduces how much time is needed to wait
        // here for the GCD to finish. Although macros always wait in 2 or
        // 3 second periods, the actual wait period is 2.0 and 2.5 seconds,
        // so that's adjusted here.
        send_action(window, &action.name);
        if action.wait == 2 {
            ui::wait_ms(1700);
        } else {
            ui::wait_ms(2200);
        };
    }
}

fn send_string(window: ui::WinHandle, s: &str) {
    log::trace!("string(`{}`)", s);
    for c in s.chars() {
        ui::send_char(window, c);
    }
}

fn send_action(window: ui::WinHandle, action: &str) {
    log::debug!("action(`{}`)", action);
    ui::enter(window);
    send_string(window, &format!("/ac \"{}\"", action));
    ui::wait_ms(50);
    ui::enter(window);
}

fn change_gearset(window: ui::WinHandle, gearset: u64) {
    log::debug!("gearset({})", gearset);
    println!("changing to gearset {}", gearset);
    ui::enter(window);
    send_string(window, &format!("/gearset change {}", gearset));
    ui::wait_ms(50);
    ui::enter(window);
}

fn toggle_collectable(window: ui::WinHandle) {
    send_action(window, &"collectable synthesis");
}

pub fn aaction(window: ui::WinHandle, verb: &str, action: &str) {
    ui::enter(window);
    if verb == "clear" {
        send_string(window, "/aaction clear");
    } else {
        send_string(window, &format!("/aaction \"{}\" {}", action, verb));
    }
    ui::enter(window);
    //ui::wait_secs(1);
}

pub fn aaction_clear(window: ui::WinHandle) {
    aaction(window, "clear", "")
}

pub fn aaction_add(window: ui::WinHandle, action: &str) {
    aaction(window, "on", action)
}

pub fn aaction_remove(window: ui::WinHandle, action: &str) {
    aaction(window, "off", action)
}
