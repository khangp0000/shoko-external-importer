# Shoko external importer

### Introduction

A very simple program that use to hard link files from one folder to another. It assume that files does not change (i.e. file is not linked again if got overwrite). Linked files is stored in a sqlite database. My main purpose is run this periodically to hard link my anime in `Sonarr` into `Shoko` drop folder for better metadata grouping on jellyfin.

### Improvement

- Maybe add a watcher for new file to convert this into a local service instead of running periodically.
- Source is made in a way that concurrency can be easily added, but I haven't seen any performance change if concurrent is used. 

### Usage
```
Simple program to import shoko anime externally

Usage: shoko_external_importer [OPTIONS] --watch-dirs <WATCH_DIRS>... --shoko-drop-dir <SHOKO_DROP_DIR>

Options:
  -w, --watch-dirs <WATCH_DIRS>...       List of directories to watch, comma separated (ex: /path/1:/path/2) [env: WATCH_DIRECTORIES=]
  -d, --data-dir <DATA_DIR>              Data directory [env: DATA_DIRECTORY=] [default: ./.shoko-external-importer]
  -s, --shoko-drop-dir <SHOKO_DROP_DIR>  Drop directory of shoko server [env: SHOKO_DROP_DIRECTORY=]
  -h, --help                             Print help (see more with '--help')
  -V, --version                          Print version
```