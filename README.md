# Shoko external importer

### **Introduction**

A very simple program that use to hard link files from one (or more) folder to another. It assume that files does not change (i.e. file is not linked again if got overwrite). Linked files is stored in a sqlite database. My main purpose is run this periodically to hard link my anime in `Sonarr` into `Shoko` drop folder for better metadata grouping on jellyfin.

### **Improvement**

- ~~Maybe add a watcher for new file to convert this into a local file watching service instead of running periodically.~~
- ~~Source is made in a way that concurrency can be easily added, but I haven't seen any performance change if concurrent is used.~~



### **Usage:** `shoko_external_importer [OPTIONS]`

#### **Options:**

* `-w`, `--watch-dirs <WATCH_DIRS>` — List of directories to watch, comma separated (ex: /path/1:/path/2)
* `-d`, `--data-dir <DATA_DIR>` — Data directory

  Default value: `./.shoko-external-importer`
* `-s`, `--shoko-drop-dir <SHOKO_DROP_DIR>` — Drop directory of shoko server
* `--daemon` — If true, run in daemon mode, not running initial scan, only check for new files

  Default value: `false`
* `-i`, `--init-run` — Only check if in daemon mode, if set, run initial scan before running watch mode

  Default value: `false`
* `--markdown-help` — Print help in markdown
* `-l`, `--log-level <LOG_LEVEL>` — Logging level

  Default value: `info`

  Possible values: `off`, `debug`, `info`, `warn`, `error`, `trace`

* `-p`, `--parallel <PARALLEL>` — Number of file processed at the same time per source directory

  Default value: `8`



<hr/>

<small><i>
    This usage was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>