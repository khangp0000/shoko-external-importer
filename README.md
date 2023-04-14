# Shoko external importer

### **Introduction**

A very simple program that use to hard link files from one (or more) folder to another. It assume that files does not change (i.e. file is not linked again if got overwrite). Linked files is stored in a sqlite database. My main purpose is run this periodically to hard link my anime in `Sonarr` into `Shoko` drop folder for better metadata grouping on jellyfin.

### **Improvement**

- Maybe add a watcher for new file to convert this into a local file watching service instead of running periodically.
- Source is made in a way that concurrency can be easily added, but I haven't seen any performance change if concurrent is used. 


### **Usage**

`shoko_external_importer [OPTIONS]`

#### **Options:**

* `-w`, `--watch-dirs <WATCH_DIRS>` — List of directories to watch, comma separated (ex: /path/1:/path/2)
* `-d`, `--data-dir <DATA_DIR>` — Data directory

  Default value: `./.shoko-external-importer`
* `-s`, `--shoko-drop-dir <SHOKO_DROP_DIR>` — Drop directory of shoko server
* `-r`, `--repeat-run-time <REPEAT_RUN_TIME>` — Repeat run scan duration, run instead of exit, run only once if not set. Sleep duration depend on this duration subtract the duration that scan took. If scan take longer than this duration, then scan will repeat immediately without sleeping
* `--markdown-help` — Print help in markdown



<hr/>

<small><i>
    This usage was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>