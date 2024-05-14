# bitcoin-bencher

This project aims to provide a simple framework to run long-running bitcoin core benchmarks.
Benchmarks can be added in config.toml and will then be run in series.

Results are stored in an sqlite db, and plotting results is planned next.

## Functional tests

Functional tests use tmpdir configured by default to `/mnt/tmp`, add add tmpdir accordingly to `/etc/fstab` (or similar):

```bash
tmpfs   /mnt/tmp    tmpfs   size=16g,nosuid,nodev   0  0
```

## Command-Line Help for `bitcoin-bencher`

This document contains the help content for the `bitcoin-bencher` command-line program.

**Command Overview:**

* [`bitcoin-bencher`↴](#bitcoin-bencher)
* [`bitcoin-bencher bench`↴](#bitcoin-bencher-bench)
* [`bitcoin-bencher bench run`↴](#bitcoin-bencher-bench-run)
* [`bitcoin-bencher bench run once`↴](#bitcoin-bencher-bench-run-once)
* [`bitcoin-bencher bench run daily`↴](#bitcoin-bencher-bench-run-daily)
* [`bitcoin-bencher graph`↴](#bitcoin-bencher-graph)
* [`bitcoin-bencher graph generate`↴](#bitcoin-bencher-graph-generate)

### `bitcoin-bencher`

Benchmarker which uses /usr/bin/time to benchmark long-running processes, and stores their results in a simple sqlite db

**Usage:** `bitcoin-bencher [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `bench` — Handle benchmark-related commands
* `graph` — Handle graph-related commands

###### **Options:**

* `--config-file <CONFIG_FILE>` — Path to bitcoin-bench config file (toml)
* `--bench-data-dir <BENCH_DATA_DIR>` — Path to the bitcoin-bench database directory

  Default value: `/Users/will/Library/.config/bench_bitcoin`
* `--bench-db-name <BENCH_DB_NAME>` — The bitcoin-bench database name

  Default value: `db.sqlite`
* `--bitcoin-data-dir <BITCOIN_DATA_DIR>` — Data dir to use for bitcoin core during tests

  Default value: `/var/folders/5z/smg2gft15pzft406cg662tdc0000gn/T/bench.Ud0sH2EytCaM`

* `--markdown-help`

  Possible values: `true`, `false`



### `bitcoin-bencher bench`

Handle benchmark-related commands

**Usage:** `bitcoin-bencher bench <COMMAND>`

###### **Subcommands:**

* `run` — Command to run benchmarks



### `bitcoin-bencher bench run`

Command to run benchmarks

**Usage:** `bitcoin-bencher bench run <COMMAND>`

###### **Subcommands:**

* `once` — Run benchmarks once
* `daily` — Run benchmarks daily between the start and end dates



### `bitcoin-bencher bench run once`

Run benchmarks once

**Usage:** `bitcoin-bencher bench run once [OPTIONS] <SRC_DIR>`

###### **Arguments:**

* `<SRC_DIR>` — Path to bitcoin source code directory

###### **Options:**

* `--date <DATE>` — Date in unix time to run tests at
* `--commit <COMMIT>` — Commit hash to run tests at



### `bitcoin-bencher bench run daily`

Run benchmarks daily between the start and end dates

**Usage:** `bitcoin-bencher bench run daily <START> <END> <SRC_DIR>`

###### **Arguments:**

* `<START>` — Start date for daily benchmarks in YYYY-MM-DD format
* `<END>` — End date for daily benchmarks in YYYY-MM-DD format
* `<SRC_DIR>` — Path to bitcoin source code directory



### `bitcoin-bencher graph`

Handle graph-related commands

**Usage:** `bitcoin-bencher graph <COMMAND>`

###### **Subcommands:**

* `generate` — Command to generate graphs



### `bitcoin-bencher graph generate`

Command to generate graphs

**Usage:** `bitcoin-bencher graph generate`


