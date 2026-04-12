# winext

A WinFsp driver for ext4 filesystems.

# Dependencies

- Rust
- WinFsp

# How to Run

```
cargo run
```

# FAQ

**Why not use WSL?**

WSL2 only supports mounting partitions on different drives via `--mount`. Ext4 on the same drive as Windows isn't supported.

**Why not use Ext4Fsd?**

Reading works, but write performance is painfully slow due to the way it handles writes.
