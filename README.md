# bitcoin-bencher

This project aims to provide a simple framework to run long-running bitcoin core benchmarks.
Benchmarks can be added in config.toml and will then be run in series.

Results are stored in an sqlite db, and plotting results is planned next.

## Functional tests

Functional tests use tmpdir configured by default to `/mnt/tmp`, add add tmpdir accordingly to `/etc/fstab` (or similar):

```bash
tmpfs   /mnt/tmp    tmpfs   size=16g,nosuid,nodev   0  0
```
