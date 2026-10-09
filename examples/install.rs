//! A pretend package install, drawn the way `uv tool install` draws one: a
//! spinner while resolving, a bar per download, then what changed.

use std::time::{Duration, Instant};

use kiln::install::{self, Change, Live};

const PACKAGES: [(&str, &str, u64); 6] = [
    ("aiohttp", "3.14.3", 1_800_000),
    ("httpx", "0.28.1", 420_000),
    ("litellm", "1.103.2", 27_230_000),
    ("pydantic-core", "2.46.5", 2_100_000),
    ("tokenizers", "0.23.2", 3_000_000),
    ("harbor", "0.23.0", 640_000),
];

fn main() {
    let started = Instant::now();
    let live = Live::start("Resolving dependencies...");
    std::thread::sleep(Duration::from_millis(900));
    live.println(&install::summary(
        "Resolved",
        &format!("{} packages", PACKAGES.len()),
        started.elapsed(),
    ));

    let prepared = Instant::now();
    live.set_message("Preparing packages...");
    let workers: Vec<_> = PACKAGES
        .iter()
        .map(|&(name, _, size)| {
            let bar = live.bar(name, size);
            std::thread::spawn(move || {
                let step = size / 40;
                for _ in 0..40 {
                    std::thread::sleep(Duration::from_millis(30 + size / 1_000_000 * 4));
                    bar.inc(step);
                }
                bar.finish();
            })
        })
        .collect();
    for (done, worker) in workers.into_iter().enumerate() {
        let _ = worker.join();
        live.set_count(done + 1, PACKAGES.len());
    }
    live.finish();
    install::emit(&install::summary(
        "Prepared",
        &format!("{} packages", PACKAGES.len()),
        prepared.elapsed(),
    ));
    install::emit(&install::summary(
        "Installed",
        &format!("{} packages", PACKAGES.len()),
        Duration::from_millis(174),
    ));
    for (name, version, _) in PACKAGES {
        install::emit(&install::change(Change::Added, name, version));
    }
}
