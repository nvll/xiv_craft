mod config;
mod craft;
mod gui;
mod macros;
mod task;

use clap::{App, Arg};
use config::write_config;
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

    let level = match matches.occurrences_of("verbose") {
        1 => log::Level::Debug,
        2 => log::Level::Trace,
        _ => log::Level::Info,
    };

    simple_logger::init_with_level(level)?;

    // These are only read at startup.
    let macros = macros::get_macro_list()?;
    log::info!("Scanning macros:");
    for m in &macros {
        log::info!("\t{}", m.name);
    }

    let mut cfg = config::read_config();
    // If we cached any task info but the user doesn't want it anymore then it
    // needs to be cleared out.
    if !cfg.options.reload_tasks {
        cfg.tasks.clear();
        if write_config(&cfg).is_err() {
            log::error!("failed to write config");
        }
    } else {
        // If we restored tasks from a saved config and the macro count changed
        // then the id may not be valid anymore.
        for task in &mut cfg.tasks {
            if task.macro_id >= macros.len() as i32 {
                task.macro_id = 0;
            }
        }
    }

    let mut handle = xiv::init()?;
    if matches.subcommand_matches("debug1").is_some() {
        return debug1_test(handle, &cfg);
    }

    if matches.subcommand_matches("debug2").is_some() {
        return debug2_test(handle, &cfg);
    }


    loop {
        let mut gui = gui::Gui::new(&macros);
        if !gui.start(&mut cfg)? {
            break;
        }

        handle.use_slow_navigation = cfg.options.use_slow_navigation;
        craft_items(handle, &cfg, &macros[..]);
    }
    println!("exiting...");
    Ok(())
}

fn debug1_test(handle: xiv::XivHandle, cfg: &config::Config) -> Result<(), Error> {
    if let Some(recipe) = get_recipe_for_job("iron ingot", 1 /* BSM */)? {
        let task = Task {
            quantity: 1,
            is_collectable: false,
            recipe,
            use_any_mats: true,
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
            use_any_mats: true,
            mat_quality: vec![MaterialCount { nq: 4, hq: 0 }],
            macro_id: 0,
        };
        craft::change_gearset(handle, cfg.gear[task.recipe.job as usize]);
        craft::toggle_collectable(handle);
        craft::select_recipe(handle, &task);
    }
    Ok(())
}
