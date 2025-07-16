## Lynn_sundial

[![Crates.io](https://img.shields.io/crates/v/lynn_sundial)](https://crates.io/crates/lynn_sundial)  [![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/cherish-ltt/lynn_tcp/blob/main/LICENSE) [![doc](https://docs.rs/lynn_sundial/badge.svg)](https://docs.rs/lynn_sundial/latest/lynn_sundial/) [![Downloads](https://img.shields.io/crates/d/lynn_sundial.svg)](https://crates.io/crates/lynn_sundial)

`Lynn_sundial` 是一个支持cron的异步并发定时任务管理器

------

### 特点

- **cron**: 基于cron库，支持cron解析

- **async**: 基于tokio的异步任务

- **reactor**: reactor模型设计

  > **适用场景**: `Lynn_sundial`采用时间轮+异步线程的设计，适用于高并发(同一时间点有多个任务需执行)、大规模(1000任务以上)，以O(1)复杂度实现高性能调度，但需容忍预分配内存的轻微浪费

### 如何使用

#### Dependencies

**default features**

default features包含：`schedule`

使用 `cargo add lynn_sundial` 或者在`Cargo.toml`添加如下:

```rust
[dependencies]
lynn_sundial = "1"
```

```rust
use crate::schedule::*;
use chrono::Local;

#[tokio::main]
async fn main() {
    let mut scheduler = Scheduler::new();
    let _ = scheduler.push_task("* * * * * ? ", println_time, RepeatModel::Repetition);
    loop {}
}

async fn println_time() {
    let now_time = Local::now();
    println!("-> {}", now_time);
}

/*
-> 2025-07-16 08:18:21.736769200 +08:00
-> 2025-07-16 08:18:22.751395400 +08:00
-> 2025-07-16 08:18:23.761559300 +08:00
-> 2025-07-16 08:18:24.773367100 +08:00
-> 2025-07-16 08:18:25.786837400 +08:00
-> 2025-07-16 08:18:26.799391800 +08:00
-> 2025-07-16 08:18:27.812838100 +08:00
-> 2025-07-16 08:18:28.825843800 +08:00
-> 2025-07-16 08:18:29.836692700 +08:00
-> 2025-07-16 08:18:30.839692700 +08:00
......
*/
```

### 路线

#### 核心功能

 ✅ 解析cron的定时异步任务

#### 扩展功能

- [ ] 可自定义任务的函数参数

> Note:
>
> 如果没有明确标识功能在什么版本中开始支持，说明功能还在开发中

### 版本日志

[version.md](https://github.com/cherish-ltt/lynn_sundial/blob/main/version.md)

### License

MIT license

### 贡献

除非您另有明确说明，否则您有意提交以包含在Lynn_sundial中的任何贡献都应被许可为MIT，无需任何额外的条款或条件。

