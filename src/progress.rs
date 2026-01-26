use console::Term;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct Pipeline {
    mp: MultiProgress,
    total: usize,
    current: usize,
    is_tty: bool,
}

impl Pipeline {
    pub fn new(total_steps: usize) -> Self {
        Self {
            mp: MultiProgress::new(),
            total: total_steps,
            current: 0,
            is_tty: Term::stderr().is_term(),
        }
    }

    pub fn start_step(&mut self, name: &str) -> StepHandle {
        self.current += 1;
        let prefix = format!("[{}/{}]", self.current, self.total);

        if self.is_tty {
            let pb = self.mp.add(ProgressBar::new_spinner());
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template(&format!("{prefix} {{spinner:.cyan}} {{msg}}"))
                    .expect("invalid spinner template"),
            );
            pb.set_message(name.to_string());
            pb.enable_steady_tick(std::time::Duration::from_millis(80));
            StepHandle::Spinner(pb)
        } else {
            eprint!("{prefix} {name}...");
            StepHandle::Plain
        }
    }

    pub fn finish_step(handle: &StepHandle, result: &str) {
        match handle {
            StepHandle::Spinner(pb) => {
                pb.finish_and_clear();
                eprintln!("  \x1b[32m✓\x1b[0m {result}");
            }
            StepHandle::Plain => {
                eprintln!(" {result}");
            }
        }
    }

    pub fn finish(&self, output_path: &str) {
        if self.is_tty {
            eprintln!("\n  \x1b[1;32m✓\x1b[0m Binary ready: {output_path}\n");
        } else {
            eprintln!("\nDone: {output_path}");
        }
    }

    pub fn mp(&self) -> &MultiProgress {
        &self.mp
    }
}

pub enum StepHandle {
    Spinner(ProgressBar),
    Plain,
}
