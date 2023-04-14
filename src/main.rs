mod db_utils;
mod io_utils;
mod models;
mod schema;

use std::{path::PathBuf, sync::Arc, thread, time::Instant};

use anyhow::{anyhow, Result};
use clap::Parser;
use db_utils::FileProcessingSqliteDb;
use io_utils::link_file;
use log::{error, info, LevelFilter, debug};
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
    #[arg(short, long, required_unless_present = "markdown_help", env = "SHOKO_DROP_DIRECTORY")]
    shoko_drop_dir: Option<PathBuf>,

    /// Repeat run scan duration, run instead of exit, run only once if not set. 
    /// Sleep duration depend on this duration subtract the duration that scan took. 
    /// If scan take longer than this duration, then scan will repeat immediately 
    /// without sleeping.
    #[arg(short, long, env = "REPEAT_RUN_TIME")]
    repeat_run_time: Option<humantime::Duration>,

    /// Print help in markdown
    #[arg(long, hide = true)]
    markdown_help: bool,
}

fn main() -> Result<()> {
    init_logger()?;
    let args = Args::parse();
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
        .collect();
    let canon_shoko_drop_dir = Arc::new(io_utils::chk_canon_dir_exists(&args.shoko_drop_dir.unwrap())?);
    let data_db_path = io_utils::chk_canon_dir_exists(&args.data_dir)?.join("db.sqlite3");

    let db = FileProcessingSqliteDb::create_from_file(&data_db_path)?;
    {
        let db_conn = db.create_connection()?;
        db_conn.run_migrations()?;
    }

    match args.repeat_run_time {
        Some(duration) => {
            let duration_between_run = *duration;
            loop {
                let start_time = Instant::now();
                run_once(&db, &canon_watch_dirs, &canon_shoko_drop_dir)?;
                let elapse_time = start_time.elapsed();
                match duration_between_run.checked_sub(elapse_time) {
                    Some(sleep_time) => {
                        info!("Sleeping for: {}", humantime::format_duration(sleep_time));
                        thread::sleep(sleep_time)
                    }
                    None => (),
                };
            }
        }
        None => Ok(run_once(&db, &canon_watch_dirs, &canon_shoko_drop_dir)?),
    }
}

fn run_once(
    db: &FileProcessingSqliteDb,
    canon_watch_dirs: &Vec<PathBuf>,
    canon_shoko_drop_dir: &PathBuf,
) -> Result<()> {
    info!("Start running scan and import, destination directory: {}", &canon_shoko_drop_dir.display());
    let pattern = "**/*.[mM][kK][vV]";
    Ok(for src_base_dir in canon_watch_dirs {
        info!("Processing folder {} with glob pattern {}", src_base_dir.display(), pattern);
        for src_file_res in globmatch::Builder::new(pattern)
            .build(&src_base_dir)
            .map_err(|err| anyhow!(err))?
        {
            match src_file_res {
                Ok(src_file) => {
                    process_file_log_only(&db, &src_file, &src_base_dir, canon_shoko_drop_dir)
                }
                Err(e) => {
                    error!("Glob result error: {}", e);
                }
            };
        }
    })
}

fn process_file_log_only(
    db: &FileProcessingSqliteDb,
    src_file: &PathBuf,
    src_base_dir: &PathBuf,
    dst_base_dir: &PathBuf,
) {
    match process_file(db, src_file, src_base_dir, dst_base_dir) {
        Ok(true) => debug!(
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

fn process_file(
    db: &FileProcessingSqliteDb,
    src_file: &PathBuf,
    src_base_dir: &PathBuf,
    dst_base_dir: &PathBuf,
) -> Result<bool> {
    let db_conn = db.create_connection()?;
    match db_conn.is_path_processed(src_file)? {
        false => link_file(src_base_dir, src_file, dst_base_dir)
            .and_then(|_| db_conn.add_processed_file(src_file))
            .map(|_| true),
        true => Ok(false),
    }
}

fn init_logger() -> Result<()> {
    Ok(TermLogger::init(
        LevelFilter::Info,
        ConfigBuilder::new().set_time_format_rfc2822().build(),
        TerminalMode::Stdout,
        ColorChoice::Auto,
    )?)
}
