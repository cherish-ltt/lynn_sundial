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
use chrono::Local;
use lynn_sundial::schedule_api::*;

#[tokio::main]
async fn main() {
    let mut scheduler = Scheduler::new();
    let _ = scheduler.push_task(
        "0/1 * * * * ?",
        println_second_time,
        RepeatModel::Repetition,
    );
    for _ in 0..5 {
        let _ = scheduler.push_task(
            "0 0/1 * * * ?",
            println_minute_time,
            RepeatModel::Repetition,
        );
    }
    scheduler.wait_all().await
}

async fn println_second_time() {
    let now_time = Local::now();
    println!("second -> {}", now_time);
}

async fn println_minute_time() {
    let now_time = Local::now();
    println!("minute -> {}", now_time);
}

/*
second -> 2025-08-14 00:17:55.068137600 +08:00
second -> 2025-08-14 00:17:56.081903600 +08:00
second -> 2025-08-14 00:17:57.092010100 +08:00
second -> 2025-08-14 00:17:58.102547000 +08:00
second -> 2025-08-14 00:17:59.128056200 +08:00
second -> 2025-08-14 00:18:00.030976200 +08:00
minute -> 2025-08-14 00:18:00.039900100 +08:00
minute -> 2025-08-14 00:18:00.039911800 +08:00
minute -> 2025-08-14 00:18:00.039935000 +08:00
minute -> 2025-08-14 00:18:00.039948400 +08:00
minute -> 2025-08-14 00:18:00.039972800 +08:00
second -> 2025-08-14 00:18:01.039895500 +08:00
second -> 2025-08-14 00:18:02.067087500 +08:00
second -> 2025-08-14 00:18:03.077046300 +08:00
second -> 2025-08-14 00:18:04.088021900 +08:00
second -> 2025-08-14 00:18:05.099003300 +08:00
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

