//! A task runner. The plan and a spinner stay live in the pane while each step
//! runs; finished steps and their output land in scrollback, and the final plan
//! is printed once everything has settled.

use std::time::{Duration, Instant};

use anyhow::Result;

use kiln::cells::{self, StepStatus};
use kiln::guard::TuiGuard;
use kiln::render::{Column, Inset, Insets, Renderable};
use kiln::status::StatusWidget;
use kiln::terminal::{Tui, TuiEvent, restore_raw};
use kiln::{text, theme};

struct Step {
    title: &'static str,
    actions: &'static [&'static str],
    output: &'static str,
    failed: bool,
}

const STEPS: [Step; 4] = [
    Step {
        title: "Fetch dependencies",
        actions: &["Read Cargo.toml", "Resolved 42 crates"],
        output: "",
        failed: false,
    },
    Step {
        title: "Build the workspace",
        actions: &["Compiled kiln", "Compiled examples"],
        output: "",
        failed: false,
    },
    Step {
        title: "Run the tests",
        actions: &[],
        output: "running 116 tests\ntest result: ok. 116 passed; 0 failed",
        failed: false,
    },
    Step {
        title: "Check formatting",
        actions: &[],
        output: "Diff in src/menu.rs at line 88:\n-        let style = if selected {\n+        let style = match selected {",
        failed: true,
    },
];

#[tokio::main]
async fn main() -> Result<()> {
    if let Some(entry) = theme::named("ember") {
        theme::set(entry.theme);
        theme::settle();
    }
    let _guard = TuiGuard::install(restore_raw);
    let mut tui = Tui::new(8)?;
    let started = Instant::now();
    let mut status: Vec<StepStatus> = vec![StepStatus::Pending; STEPS.len()];
    let mut spinner = StatusWidget::new(tui.frame_requester());

    for (index, step) in STEPS.iter().enumerate() {
        status[index] = StepStatus::InProgress;
        spinner.set_detail(Some(step.title.to_string()));
        let actions = step.actions.len().max(1);
        for action in 0..actions {
            let until = Instant::now() + Duration::from_millis(900);
            while Instant::now() < until {
                let mut pane = Column::new();
                pane.push(cells::plan(&labelled(&status), tui.width() as usize));
                pane.push(Inset::new(&spinner, Insets::tlbr(1, 2, 0, 0)));
                tui.draw(pane.desired_height(tui.width()), |frame| {
                    pane.render(frame.area(), frame.buffer_mut());
                })?;
                tokio::select! {
                    event = tui.next_event() => match event {
                        TuiEvent::Resize => tui.resized()?,
                        TuiEvent::Key(_) | TuiEvent::Mouse(_) | TuiEvent::Paste(_) | TuiEvent::Draw => {}
                    },
                    _ = tokio::time::sleep_until(until.into()) => {}
                }
            }
            let width = tui.width() as usize;
            if let Some(label) = step.actions.get(action) {
                tui.insert_history(cells::group_row(step.title, label, action > 0, width))?;
            }
        }
        let width = tui.width() as usize;
        if !step.output.is_empty() {
            tui.insert_history(cells::tool(step.title, step.output, step.failed, width))?;
        }
        status[index] = match step.failed {
            true => StepStatus::Blocked,
            false => StepStatus::Done,
        };
    }

    let width = tui.width() as usize;
    tui.insert_history(cells::plan(&labelled(&status), width))?;
    tui.insert_history(cells::error_box(
        &["Formatting check failed. Run cargo fmt, then try again.".into()],
        width,
    ))?;
    tui.insert_history(cells::notice(
        &format!("Finished in {}", text::elapsed(started.elapsed().as_secs())),
        width,
    ))?;
    Ok(())
}

fn labelled(status: &[StepStatus]) -> Vec<(StepStatus, String)> {
    STEPS
        .iter()
        .zip(status)
        .map(|(step, status)| (*status, step.title.to_string()))
        .collect()
}
