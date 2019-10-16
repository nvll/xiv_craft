mod config;
mod craft;
mod gui;
mod lists;
mod macros;
mod recipe;
mod rpc;
mod task;

use failure::Error;
use rpc::{Request, Response, Worker};
use simple_logger;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "talan", about = "A FFXIV Crafting helper")]
struct Opts {
    /// Enable log levels
    #[structopt(short = "v", parse(from_occurrences))]
    verbose: u64,
}

fn parse_arguments() -> Result<(), Error> {
    let args = Opts::from_args();
    simple_logger::init_with_level(match args.verbose {
        1 => log::Level::Debug,
        2 => log::Level::Trace,
        _ => log::Level::Info,
    })?;

    Ok(())
}

fn main() -> Result<(), Error> {
    parse_arguments()?;

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
