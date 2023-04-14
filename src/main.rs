mod db_utils;
mod io_utils;
mod models;
mod schema;

use std::{path::PathBuf, sync::Arc};

use anyhow::{anyhow, Result};
use clap::Parser;
use db_utils::FileProcessingSqliteDb;
use io_utils::link_file;
use log::{error, info, LevelFilter};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};

/// Simple program to import shoko anime externally
#[derive(Parser, Debug)]
#[command(author, version)]
struct Args {
    /// List of directories to watch, comma separated (ex: /path/1:/path/2)
    #[arg(short, long, long_help, value_delimiter = ':', required = true, num_args = 1.., env = "WATCH_DIRECTORIES")]
    watch_dirs: Vec<PathBuf>,

    /// Data directory
    #[arg(
        short,
        long,
        default_value = "./.shoko-external-importer",
        env = "DATA_DIRECTORY"
    )]
    data_dir: PathBuf,

    /// Drop directory of shoko server
    #[arg(short, long, required = true, env = "SHOKO_DROP_DIRECTORY")]
    shoko_drop_dir: PathBuf,
}

fn main() -> Result<()> {
    init_logger()?;
    let args = Args::parse();
    let canon_watch_dirs = args
        .watch_dirs
        .into_iter()
        .map(|path| io_utils::chk_canon_dir_exists(&path))
        .map(Result::unwrap);
    let canon_shoko_drop_dir = Arc::new(io_utils::chk_canon_dir_exists(&args.shoko_drop_dir)?);
    let data_db_path = io_utils::chk_canon_dir_exists(&args.data_dir)?.join("db.sqlite3");

    let db = FileProcessingSqliteDb::create_from_file(&data_db_path)?;
    {
        let db_conn = db.create_connection()?;
        db_conn.run_migrations()?;
    }

    let pattern = "**/*.[mM][kK][vV]";
    for src_base_dir in canon_watch_dirs {
        for src_file_res in globmatch::Builder::new(pattern)
            .build(&src_base_dir)
            .map_err(|err| anyhow!(err))?
        {
            match src_file_res {
                Ok(src_file) => {
                    process_file_log_only(&db, &src_file, &src_base_dir, &canon_shoko_drop_dir)
                }
                Err(e) => {
                    error!("Glob result error: {}", e);
                }
            };
        }
    }

    Ok(())
}

fn process_file_log_only(
    db: &FileProcessingSqliteDb,
    src_file: &PathBuf,
    src_base_dir: &PathBuf,
    dst_base_dir: &PathBuf,
) {
    match process_file(db, src_file, src_base_dir, dst_base_dir) {
        Ok(true) => info!(
            "Processing file successfully: {} - Destination directory: {}",
            src_file.display(),
            dst_base_dir.display()
        ),
        Ok(false) => info!("Skipping file, already processed: {} ", src_file.display()),
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
