mod config;
mod craft;
mod gui;
mod macros;
mod recipe;
mod rpc;
mod task;

use clap::{App, Arg};
use failure::Error;
use rpc::{Request, Response, Worker};
use simple_logger;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

fn main() -> Result<(), Error> {
    let matches =
        App::new("Ventures")
            .arg(
                Arg::with_name("verbose")
                    .short("v")
                    .multiple(true)
                    .help("Log level to use. Multiple can be used"),
            )
            .arg(Arg::with_name("slow").short("s").long("slow").help(
                "Run with a longer delay between UI actions. Recommended for slower computers.",
            ))
            .subcommand(
                App::new("debug1")
                    .help("Bring up the item window and attempt to search for an iron ingot"),
            )
            .subcommand(App::new("debug2").help(
                "Attempt to set up the UI to build a collectable cloud pearl as CRP (set 13)",
            ))
            .get_matches();

    let level = match matches.occurrences_of("verbose") {
        1 => log::Level::Debug,
        2 => log::Level::Trace,
        _ => log::Level::Info,
    };

    simple_logger::init_with_level(level)?;

    // These are only read at startup.
    let macros = match macros::get_macro_list() {
        Ok(m) => m,
        Err(e) => {
            log::error!(
                "Couldn't open macros directory for reading: {}",
                e.to_string()
            );
            return Ok(()); // counter-intuitive, but we want to suppress additional messages.
        }
    };

    log::info!("Scanning macros:");
    for m in &macros {
        log::info!("\t{}", m.name);
    }

    let mut cfg = config::get_config(None);
    // If we cached any task info but the user doesn't want it anymore then it
    // needs to be cleared out.
    if !cfg.options.reload_tasks {
        cfg.tasks.clear();
    } else {
        // If we restored tasks from a saved config and the macro count changed
        // then the id may not be valid anymore.
        for task in &mut cfg.tasks {
            if task.macro_id >= macros.len() as i32 {
                task.macro_id = 0;
            }
        }
    }

    let (client_tx, worker_rx): (Sender<Request>, Receiver<Request>) = channel();
    let (worker_tx, client_rx): (Sender<Response>, Receiver<Response>) = channel();
    thread::spawn(move || Worker::new(worker_rx, worker_tx).worker_thread());

    let mut gui = gui::Gui::new(&macros, &client_tx, &client_rx);
    gui.start(&mut cfg);

    println!("exiting...");
    Ok(())
}
