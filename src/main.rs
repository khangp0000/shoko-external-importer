mod db_utils;
mod file_process_manager;
mod io_utils;
mod models;
mod schema;

use std::{
    path::PathBuf,
    sync::Arc,
    thread::{spawn, JoinHandle},
    time::Duration,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use crossbeam_channel::Sender;
use db_utils::FileProcessingSqliteDb;
use file_process_manager::FileProcessingManager;
use io_utils::link_file;
use log::{debug, error, info, trace, LevelFilter};
use notify_debouncer_mini::{
    new_debouncer,
    notify::{RecommendedWatcher, RecursiveMode},
    DebounceEventResult, Debouncer,
};
use sd_notify::NotifyState;
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};

/// Simple program to import shoko anime externally
#[derive(Parser, Debug)]
#[command(author, version)]
struct Args {
    /// List of directories to watch, comma separated (ex: /path/1:/path/2)
    #[arg(short, long, long_help, value_delimiter = ':', required_unless_present = "markdown_help", num_args = 1.., env = "WATCH_DIRECTORIES")]
    watch_dirs: Option<Vec<PathBuf>>,

    /// Data directory
    #[arg(
        short,
        long,
        default_value = "./.shoko-external-importer",
        env = "DATA_DIRECTORY"
    )]
    data_dir: PathBuf,

    /// Drop directory of shoko server
    #[arg(
        short,
        long,
        required_unless_present = "markdown_help",
        env = "SHOKO_DROP_DIRECTORY"
    )]
    shoko_drop_dir: Option<PathBuf>,

    /// If true, run in daemon mode, not running initial scan, only check for new
    /// files
    #[arg(long, default_value = "false", env = "DAEMON")]
    daemon: bool,

    /// If true, notify systemd of ready status
    #[arg(long, default_value = "false", env = "SYSTEMD_NOTIFY")]
    systemd_notify: bool,

    /// Only check if in daemon mode, if set, run initial scan before running watch mode.
    #[arg(short, long, default_value = "false", env = "INIT_RUN")]
    init_run: bool,

    /// Print help in markdown
    #[arg(long, hide = true)]
    markdown_help: bool,

    /// Logging level
    #[arg(short, long, default_value = "info", env = "LOGGING_LEVEL", value_enum)]
    log_level: LogLevel,

    /// Maximum number of file processed at the same time
    #[arg(short, long, default_value = "8", env = "PARALLEL")]
    parallel: usize,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum LogLevel {
    Off,
    Debug,
    Info,
    Warn,
    Error,
    Trace,
}

fn main() -> Result<()> {
    let args = Args::parse();
    init_logger(args.log_level)?;
    if args.markdown_help {
        clap_markdown::print_help_markdown::<Args>();
        return Ok(());
    }
    let canon_watch_dirs: Vec<_> = args
        .watch_dirs
        .unwrap()
        .into_iter()
        .map(|path| io_utils::chk_canon_dir_exists(&path))
        .map(Result::unwrap)
        .map(Arc::new)
        .collect();

    let canon_shoko_drop_dir = Arc::new(io_utils::chk_canon_dir_exists(
        &args.shoko_drop_dir.unwrap(),
    )?);

    let data_db_path = io_utils::chk_canon_dir_exists(&args.data_dir)?.join("db.sqlite3");
    let db = Arc::new(FileProcessingSqliteDb::create_from_file(&data_db_path)?);
    {
        let db_conn = db.create_connection()?;
        db_conn.run_migrations()?;
    }

    let (sender, task) = init_file_processor(&db, &canon_shoko_drop_dir, args.parallel);

    if !args.daemon || args.init_run {
        run_once(&sender, &canon_watch_dirs, &canon_shoko_drop_dir)?;
    }

    let _debouncers: Vec<Debouncer<RecommendedWatcher>>; // Need to keep this alive or it will stop the notify thread
    if args.daemon {
        _debouncers = run_watch(&sender, &canon_watch_dirs, &canon_shoko_drop_dir)?;
    }

    if args.systemd_notify {
        sd_notify::notify(true, &[NotifyState::Ready]).unwrap();
    }

    drop(sender);
    Ok(task.join().unwrap())
}

fn run_once(
    sender: &Sender<(Arc<PathBuf>, Arc<PathBuf>)>,
    canon_watch_dirs: &Vec<Arc<PathBuf>>,
    canon_shoko_drop_dir: &Arc<PathBuf>,
) -> Result<()> {
    info!(
        "Start running scan and import in one time mode, destination directory: {}",
        &canon_shoko_drop_dir.display()
    );
    let pattern = "**/*.[mM][kK][vV]";
    Ok(for src_base_dir in canon_watch_dirs {
        info!(
            "Processing folder {} with glob pattern {}",
            src_base_dir.display(),
            pattern
        );
        for src_file_res in globmatch::Builder::new(pattern)
            .build(&src_base_dir.clone().as_ref())
            .map_err(|err| anyhow!(err.clone()))?
        {
            match src_file_res {
                Ok(src_file) => sender.send((Arc::new(src_file), src_base_dir.clone()))?,
                Err(e) => {
                    error!("Glob result error: {}", e);
                }
            };
        }
    })
}

fn run_watch(
    sender: &Sender<(Arc<PathBuf>, Arc<PathBuf>)>,
    canon_watch_dirs: &Vec<Arc<PathBuf>>,
    canon_shoko_drop_dir: &Arc<PathBuf>,
) -> Result<Vec<Debouncer<RecommendedWatcher>>> {
    info!(
        "Start running scan and import in watch mode, destination directory: {}",
        &canon_shoko_drop_dir.display()
    );

    let mut debouncers = Vec::new(); // save life time of debouncers

    for src_base_dir in canon_watch_dirs {
        let sender_clone = sender.clone();

        info!("Processing folder in watch mode {}", src_base_dir.display());

        let src_base_dir_clone = src_base_dir.clone();
        let mut debouncer = new_debouncer(
            Duration::from_secs(10),
            move |res: DebounceEventResult| match res {
                Result::Ok(events) => events.into_iter().for_each(|event| match event.kind {
                    notify_debouncer_mini::DebouncedEventKind::Any => {
                        if event.path.is_file() {
                            match event.path.extension() {
                                Some(ext) => {
                                    if ext.to_ascii_lowercase() == "mkv" {
                                        sender_clone
                                            .send((
                                                Arc::new(event.path),
                                                src_base_dir_clone.clone(),
                                            ))
                                            .unwrap();
                                    } else {
                                        debug!("Ignore file {}", event.path.display());
                                    }
                                }
                                None => debug!("Ignore file {}", event.path.display()),
                            }
                        } else {
                            debug!("Ignore deleted file {}", event.path.display())
                        }
                    }
                    _ => debug!("Ignore ContinuousAny event"),
                }),
                Err(errors) => error!("Debouncing error: {:?}", errors),
            },
        )
        .unwrap();

        debouncer
            .watcher()
            .watch(src_base_dir, RecursiveMode::Recursive)
            .unwrap();

        debouncers.push(debouncer);
    }

    return Ok(debouncers);
}

fn process_file(
    db: &Arc<FileProcessingSqliteDb>,
    src_file: &Arc<PathBuf>,
    src_base_dir: &Arc<PathBuf>,
    dst_base_dir: &Arc<PathBuf>,
) -> Result<bool> {
    let db_conn = db.create_connection()?;
    match db_conn.is_path_processed(&src_file)? {
        false => link_file(&src_base_dir, &src_file, &dst_base_dir)
            .and_then(|_| db_conn.add_processed_file(&src_file))
            .map(|_| true),
        true => Ok(false),
    }
}

fn init_logger(log_level: LogLevel) -> Result<()> {
    Ok(TermLogger::init(
        match log_level {
            LogLevel::Off => LevelFilter::Off,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Trace => LevelFilter::Trace,
        },
        ConfigBuilder::new().set_time_format_rfc2822().build(),
        TerminalMode::Stdout,
        ColorChoice::Auto,
    )?)
}

fn init_file_processor(
    db: &Arc<FileProcessingSqliteDb>,
    dst_base_dir: &Arc<PathBuf>,
    parallel: usize,
) -> (Sender<(Arc<PathBuf>, Arc<PathBuf>)>, JoinHandle<()>) {
    let (send, recv) = crossbeam_channel::bounded(100);
    let db_clone = db.clone();
    let dst_base_dir_clone = dst_base_dir.clone();
    let dst_base_dir_clone_2 = dst_base_dir.clone();
    let file_processing_manager = FileProcessingManager::new(
        parallel,
        recv,
        Box::new(move |f, s| process_file(&db_clone, f, s, &dst_base_dir_clone)),
        Box::new(move |f, r| result_handler(r, f, &dst_base_dir_clone_2)),
    );

    return (
        send,
        spawn(move || loop {
            match file_processing_manager.process_once() {
                Err(e) => {
                    info!("Error processing: {:?}", e);
                    break;
                }
                _ => trace!("Finish processing 1 file"),
            }
        }),
    );
}

fn result_handler(result: Result<bool>, src_file: &Arc<PathBuf>, dst_base_dir: &Arc<PathBuf>) {
    match result {
        Ok(true) => info!(
            "Processing file successfully: {} - Destination directory: {}",
            src_file.display(),
            dst_base_dir.display()
        ),
        Ok(false) => debug!("Skipping file, already processed: {} ", src_file.display()),
        Err(e) => error!(
            "Failed to process file: {} with error: {}",
            src_file.display(),
            e
        ),
    }
}
