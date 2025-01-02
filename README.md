# File Update Monitor

A Rust library for monitoring file changes. It provides a simple interface to watch file changes in directories and execute custom callback functions when file content changes.

## Main Features

- Monitor file changes in specified directories and subdirectories
- Support debouncing to avoid too frequent updates
- Asynchronous handling of file change events
- Customizable logic for handling file changes

## Installation

```shell
cargo add file_update_monitor
```

## Usage

```rust
use file_update_monitor::Monitor;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let monitor = Monitor::new("./", 1000, |path| {
        println!("File updated: {}", path);
        Ok(())
    });
    
    monitor.start().await;
    Ok(())
}
```

