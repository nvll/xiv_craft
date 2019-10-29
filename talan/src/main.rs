mod config;
mod craft;
mod gui;
mod lists;
mod macros;
mod recipe;
mod rpc;
mod task;

use env_logger;
use failure::Error;
use log;
use rpc::{Request, Response, Worker};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "talan", about = "A FFXIV Crafting helper")]
struct Opts {
    /// Path to the macro file
    #[structopt(short = "m", long = "macros", default_value = "macros.toml")]
    macros_path: PathBuf,
    /// Path to the config file
    #[structopt(short = "c", long = "config", default_value = "config.json")]
    config_path: PathBuf,
    /// Enable log levels (use multiple -v for more logging)
    #[structopt(short = "v", parse(from_occurrences))]
    verbose: u64,
}

fn parse_arguments() -> Result<config::Config, Error> {
    let args = Opts::from_args();
    env_logger::Builder::from_default_env()
        .filter(
            Some("talan"),
            match args.verbose {
                1 => log::LevelFilter::Debug,
                2 => log::LevelFilter::Trace,
                _ => log::LevelFilter::Info,
            },
        )
        .init();

    if let Err(e) = macros::from_path(&args.macros_path) {
        log::error!(
            "Failed to read macros from '{}': {}",
            &args.macros_path.to_str().unwrap(),
            e.to_string(),
        );
        std::process::exit(1);
    }

    let mut cfg = config::get_config(Some(&args.config_path));
    for task in &mut cfg.tasks {
        task.macro_id = macros::get_macro_for_recipe(
            task.recipe.durability,
            task.recipe.level,
            cfg.options.specialist[task.recipe.job as usize],
        );
    }

    Ok(cfg)
}

fn main() -> Result<(), Error> {
    let mut cfg = parse_arguments()?;
    let (client_tx, worker_rx): (Sender<Request>, Receiver<Request>) = channel();
    let (worker_tx, client_rx): (Sender<Response>, Receiver<Response>) = channel();
    thread::spawn(move || Worker::new(worker_rx, worker_tx).worker_thread());

    let mut gui = gui::Gui::new(&client_tx, &client_rx);
    gui.start(&mut cfg);

    println!("exiting...");
    Ok(())
}
