use failure::Error;
use simple_logger;
use std::thread;
use std::time::{Duration, Instant};
use structopt;
use structopt::StructOpt;
use xiv;
use xiv::ui;

#[derive(Debug, StructOpt)]
#[structopt(name = "ventures", about = "A FFXIV venture automation helper")]
struct Opts {
    /// Use slower menu navigation for slower or laggier systems
    #[structopt(short = "s", long = "slow")]
    use_slow_navigation: bool,

    /// The index of retainers to send on ventures.
    /// Retainers can be specified by ranges denoted by a hyphen, or individuals
    /// separated by commas. Ranges must be low to high.
    ///
    /// e.g. the following are the same:
    ///
    ///   --retainers 1-4
    ///   --retainers 1,2,3-4
    ///   --retainers 1,2,3,4
    #[structopt(short = "r", long = "retainers")]
    retainers: String,

    /// By default, Paissa assumes all retainers have just been sent out and will
    /// check them when the one with the shortest venture length finishes. This delay
    /// allows you to adjust the first time it checks all the retainers. For instance,
    /// if you sent all the retainers out 5 minutes ago you could use -t 5 to indicate
    /// retainers should be checked five minutes earlier.
    #[structopt(short = "t", long = "time_passed")]
    time_passed: Option<u64>,

    /// How many minutes a retainer's ventures take to complete (default:60)
    #[structopt(short = "1")]
    r1_period: Option<u64>,
    #[structopt(short = "2")]
    r2_period: Option<u64>,
    #[structopt(short = "3")]
    r3_period: Option<u64>,
    #[structopt(short = "4")]
    r4_period: Option<u64>,
    #[structopt(short = "5")]
    r5_period: Option<u64>,
    #[structopt(short = "6")]
    r6_period: Option<u64>,
    #[structopt(short = "7")]
    r7_period: Option<u64>,
    #[structopt(short = "8")]
    r8_period: Option<u64>,

    /// Enable log levels.
    #[structopt(short = "v", parse(from_occurrences))]
    verbose: u64,
}

#[derive(Debug)]
struct Retainer {
    id: u64,
    period: Duration,
    next: Instant,
}

impl Retainer {
    fn new(id: u64, args: &Opts) -> Retainer {
        let period = Duration::from_secs(retainer_id_to_period(id, args) * 60);
        Retainer {
            id,
            period,
            next: Instant::now() + period,
        }
    }
}

const DEFAULT_PERIOD: u64 = 60;

// TODO: This whole method could just be a simple macro?
#[rustfmt::skip]
fn retainer_id_to_period(id: u64, args: &Opts) -> u64 {
    match id {
        1 => args.r1_period.unwrap_or(DEFAULT_PERIOD),
        2 => args.r2_period.unwrap_or(DEFAULT_PERIOD),
        3 => args.r3_period.unwrap_or(DEFAULT_PERIOD),
        4 => args.r4_period.unwrap_or(DEFAULT_PERIOD),
        5 => args.r5_period.unwrap_or(DEFAULT_PERIOD),
        6 => args.r6_period.unwrap_or(DEFAULT_PERIOD),
        7 => args.r7_period.unwrap_or(DEFAULT_PERIOD),
        8 => args.r8_period.unwrap_or(DEFAULT_PERIOD),
        _ => panic!("Unknown ID"),
    }
}

fn parse_arguments() -> Result<(xiv::XivHandle, Vec<Retainer>), Error> {
    let args = Opts::from_args();
    simple_logger::init_with_level(match args.verbose {
        1 => log::Level::Debug,
        2 => log::Level::Trace,
        _ => log::Level::Info,
    })?;

    // Parse a mix of ranges specified by X-Y or separated by commas X,Y,Z
    let mut retainers: Vec<Retainer> = Vec::new();
    for hunk in args.retainers.split(',') {
        let v: Vec<u64> = hunk.split('-').map(|s| s.parse::<u64>().unwrap()).collect();
        retainers.push(Retainer::new(v[0], &args));
        if v.len() == 2 {
            for i in v[0] + 1..=v[1] {
                retainers.push(Retainer::new(i, &args));
            }
        }
    }

    if let Some(t) = args.time_passed {
        log::info!(
            "Adjusting all retainer initial completion times to be {}m earlier",
            t
        );

        for r in &mut retainers {
            r.next -= Duration::from_secs(t * 60);
        }
    }

    retainers.sort_by_key(|r| r.id);
    for r in &retainers {
        log::info!("retainer {} every {}m", r.id, r.period.as_secs() / 60);
    }

    let mut h = xiv::init()?;
    h.use_slow_navigation = args.use_slow_navigation;

    Ok((h, retainers))
}

fn main() -> Result<(), Error> {
    let (hnd, mut retainers) = parse_arguments()?;

    // Who knows what state the UI will be in
    ui::clear_window(hnd);
    // Open the retainer menu initially to keep from being logged out while AFK.
    open_retainer_menu(hnd);
    loop {
        // Figure out who the first retainer to be finished is and sleep until then.
        retainers.sort_by_key(|r| r.next);
        for r in &retainers {
            let nd = r.next - Instant::now();
            log::debug!(
                "Retainer {{ id: {}, period: {}, next: {}m{}s }}",
                r.id,
                r.period.as_secs() / 60,
                nd.as_secs() / 60,
                nd.as_secs() % 60
            );
        }

        if retainers[0].next > Instant::now() {
            let sleep_duration = retainers[0].next - Instant::now();
            log::info!(
                "Retainer {} is next in {}m{}s.",
                retainers[0].id,
                sleep_duration.as_secs() / 60,
                sleep_duration.as_secs() % 60
            );
            thread::sleep(sleep_duration);
        }

        // Always re-open the menu to ensure the state is consistent. This is
        // important because if the user does anything in the intervening time,
        // even simple things like tabbing to the game and out again, it may
        // change the input state and throw all our inputs off by one.
        open_retainer_menu(hnd);
        // Run any retainer that finished and update their next venture deadline.
        for r in &mut retainers {
            if r.next < Instant::now() {
                log::info!("re-assigning retainer {}'s venture", r.id);
                reassign_venture(hnd, r.id);
                log::debug!("retainer {} done", r.id);
                // Base the delay to the next venture by when we finish navigating
                // the menus. We could speed this up by 20-30 seconds, but when we're
                // working with 40-60 minute deltas it's better to be safe.
                r.next = Instant::now() + r.period;
            }
        }
    }
}

fn open_retainer_menu(hnd: xiv::XivHandle) {
    log::debug!("open_retainer_menu");
    // This will close the game menu if open and exit the retainer window if
    // it was open from a previous run.
    ui::press_escape(hnd);
    ui::press_escape(hnd);
    ui::press_cancel(hnd);
    ui::press_cancel(hnd);
    ui::wait(2.0);

    // The reason the menu is opened twice is because we want to clear out any
    // mouse actions the UI registered that would lead to us not having the input
    // cursor up when the retainer menu opens.
    ui::target_nearest_npc(hnd);
    ui::wait(1.0);
    ui::press_confirm(hnd);
    ui::wait(2.0);
    ui::press_cancel(hnd);
    ui::press_cancel(hnd);
    ui::target_nearest_npc(hnd);
    ui::wait(1.0);
    ui::press_confirm(hnd);
    ui::wait(2.0);
}

// General usability rules
// 1. Wait 1 second after moving around in a menu
// 2. Wait 2 seconds after pressing a button for UI changes / Feo Ul / Retainer dialog.
fn reassign_venture(hnd: xiv::XivHandle, r_id: u64) {
    log::debug!("reassign_venture(r_id: {})", r_id);
    for _ in 0..r_id - 1 {
        ui::cursor_down(hnd);
    }
    ui::wait(1.0);
    ui::press_confirm(hnd);
    ui::wait(2.0);
    ui::press_confirm(hnd);
    ui::wait(2.0);
    // Move down to Assign Venture / View Venture Progress
    for _ in 0..5 {
        ui::cursor_down(hnd);
    }
    ui::wait(1.0);
    // Select the menu option
    ui::press_confirm(hnd);
    ui::wait(2.0);
    // Move left to 'Reassign'
    ui::cursor_left(hnd);
    ui::wait(1.0);
    // Confirm 'Reassign'
    ui::press_confirm(hnd);
    ui::wait(2.0);
    // Move left to 'Assign' in the venture window that comes up
    ui::cursor_left(hnd);
    ui::wait(1.0);
    // Confirm 'Assign'
    ui::press_confirm(hnd);
    ui::wait(2.0);
    // Confirm the message from the retainer about the venture
    ui::press_confirm(hnd);
    ui::wait(2.0);
    // Escape out of the specific retainer's menu
    ui::press_cancel(hnd);
    ui::wait(2.0);
    // Say goodbye to the retainer
    ui::press_confirm(hnd);
    ui::wait(2.0);
}
