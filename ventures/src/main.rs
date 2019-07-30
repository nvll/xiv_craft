use chrono::{Local, Timelike};
use clap::{value_t, App, Arg};
use failure::Error;
use std::thread;
use xiv;
use xiv::ui;

fn sleep(s: u64) {
    thread::sleep(std::time::Duration::from_secs(s));
}

const VERSION: &str = "1.0";
fn main() -> Result<(), Error> {
    let matches = App::new("Ventures")
        .version(VERSION)
        .arg(
            Arg::with_name("retainers")
                .short("r")
                .takes_value(true)
                .required(true)
                .help(concat!(
                    "The retainers to automate.",
                    "Comma separators and ranges are accepted.\n",
                    "e.g. -r 1-4 or -r 1,3,4-5"
                )),
        )
        .arg(
            Arg::with_name("delay")
                .short("d")
                .takes_value(true)
                .help("The number of minutes to wait before starting."),
        )
        .get_matches();

    // Figure out retainer ranges by parsing out , and -
    let mut retainers: Vec<u32> = Vec::new();
    for hunk in matches.value_of("retainers").unwrap().split(',') {
        let v: Vec<u32> = hunk.split('-').map(|s| s.parse::<u32>().unwrap()).collect();
        retainers.push(v[0]);
        if v.len() == 2 {
            for i in v[0] + 1..=v[1] {
                retainers.push(i)
            }
        }
    }

    println!(
        "Reassigning ventures for retainer{} {:?}",
        if retainers.len() > 1 { "s" } else { "" },
        retainers
    );

    let mut delay_m: u64 = 0;
    if matches.occurrences_of("delay") > 0 {
        delay_m = value_t!(matches.value_of("delay"), u64).unwrap_or_else(|e| e.exit());
    }
    let h = xiv::init(false)?;

    loop {
        let mut now = Local::now();
        if delay_m > 0 {
            println!(
                "[{:02}:{:02}:{:02}] Waiting {} minutes before checking retainers",
                now.hour(),
                now.minute(),
                now.second(),
                delay_m
            );
            // Open the menu initially because we likely want to keep the player from
            // being logged out due to the 30 minute afk timer.
            open_retainer_menu(h);
            sleep(delay_m * 60);
            delay_m = 0;
        }

        // Always re-open the menu to ensure the state is consistent. This is
        // important because if the user does anything in the intervening time,
        // even simple things like tabbing to the game and out again, it may
        // change the input state and throw all our inputs off by one.
        open_retainer_menu(h);
        for r in &retainers {
            reassign_venture(h, *r);
        }

        now = Local::now();
        let next = now + chrono::Duration::minutes(60);
        println!(
            "[{:02}:{:02}:{:02}] Run finished. Next is at {:02}:{:02}:{:02}",
            now.hour(),
            now.minute(),
            now.second(),
            next.hour(),
            next.minute(),
            next.second(),
        );
        sleep(60 * 60); // 60 minutes
    }
}

fn open_retainer_menu(h: xiv::XivHandle) {
    // This will close the game menu if open and exit the retainer window if
    // it was open from a previous run.
    ui::press_escape(h);
    ui::press_escape(h);
    ui::press_cancel(h);
    ui::press_cancel(h);
    sleep(2);

    // The reason the menu is opened twice is because we want to clear out any
    // mouse actions the UI registered that would lead to us not having the input
    // cursor up when the retainer menu opens.
    ui::target_nearest_npc(h);
    sleep(1);
    ui::press_confirm(h);
    sleep(2);
    ui::press_cancel(h);
    ui::press_cancel(h);
    ui::target_nearest_npc(h);
    sleep(1);
    ui::press_confirm(h);
    sleep(2);
}

// General usability rules
// 1. Wait 1 second after moving around in a menu
// 2. Wait 2 seconds after pressing a button for UI changes / Feo Ul / Retainer dialog.
fn reassign_venture(h: xiv::XivHandle, r_id: u32) {
    println!("Selecting Retainer #{}", r_id);
    for _ in 0..r_id - 1 {
        ui::cursor_down(h);
    }
    sleep(1);
    ui::press_confirm(h);
    sleep(2);
    ui::press_confirm(h);
    sleep(2);
    // Move down to Assign Venture / View Venture Progress
    for _ in 0..5 {
        ui::cursor_down(h);
    }
    sleep(1);
    // Select the menu option
    ui::press_confirm(h);
    sleep(2);
    // Move left to 'Reassign'
    ui::cursor_left(h);
    sleep(1);
    // Confirm 'Reassign'
    ui::press_confirm(h);
    sleep(2);
    // Move left to 'Assign' in the venture window that comes up
    ui::cursor_left(h);
    sleep(1);
    // Confirm 'Assign'
    ui::press_confirm(h);
    sleep(2);
    // Confirm the message from the retainer about the venture
    ui::press_confirm(h);
    sleep(2);
    // Escape out of the specific retainer's menu
    ui::press_cancel(h);
    sleep(2);
    // Say goodbye to the retainer
    ui::press_confirm(h);
    sleep(2);
}
