mod config;
mod craft;
mod gui;
mod macros;
//mod role_actions;
mod task;

use clap::{App, Arg};
use craft::craft_items;
use failure::Error;
use simple_logger;
use task::{MaterialCount, Task};
use xivapi::get_recipe_for_job;

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

    let mut level: log::Level = log::Level::Error;
    match matches.occurrences_of("verbose") {
        1 => level = log::Level::Info,
        2 => level = log::Level::Debug,
        3 => level = log::Level::Trace,
        _ => (),
    }

    simple_logger::init_with_level(level)?;

    let handle = xiv::init(matches.is_present("slow"))?;
    // Read here, but can be updated by a UI impl.
    let mut cfg = config::read_config();
    // Any UI implementation will populate this vec.
    let mut tasks: Vec<Task> = Vec::new();
    // These are only read at startup.
    let macros = macros::get_macro_list()?;
    log::info!("Scanning macros:");
    for m in &macros {
        log::info!("\t{}", m.name);
    }

    if matches.subcommand_matches("debug1").is_some() {
        return debug1_test(handle, &cfg);
    }

    if matches.subcommand_matches("debug2").is_some() {
        return debug2_test(handle, &cfg);
    }

    if gui::start(&mut cfg, &mut tasks, &macros)? {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        craft_items(handle, &cfg, &tasks[..], &macros[..]);
    } else {
        println!("exiting...");
    }
    Ok(())
}

fn debug1_test(handle: xiv::XivHandle, cfg: &config::Config) -> Result<(), Error> {
    if let Some(recipe) = get_recipe_for_job("iron ingot", 1 /* BSM */)? {
        let task = Task {
            quantity: 1,
            is_collectable: false,
            recipe,
            ignore_mat_quality: true,
            mat_quality: vec![MaterialCount { nq: 4, hq: 0 }],
            macro_id: 0,
        };
        craft::change_gearset(handle, cfg.gear[task.recipe.job as usize]);
        craft::select_recipe(handle, &task);
    }
    Ok(())
}

// Used for testing general UI functionality
fn debug2_test(handle: xiv::XivHandle, cfg: &config::Config) -> Result<(), Error> {
    if let Some(recipe) = get_recipe_for_job("clOud PeArl", 0 /* CRP */)? {
        let task = Task {
            quantity: 1,
            is_collectable: false,
            recipe,
            ignore_mat_quality: true,
            mat_quality: vec![MaterialCount { nq: 4, hq: 0 }],
            macro_id: 0,
        };
        craft::change_gearset(handle, cfg.gear[task.recipe.job as usize]);
        craft::toggle_collectable(handle);
        craft::select_recipe(handle, &task);
    }
    Ok(())
}
