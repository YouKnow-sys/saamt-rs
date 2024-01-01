use std::io::{stdout, BufWriter, StdoutLock, Write};

use saamt_core::reporter::{Logger, ProgressReport};

use crate::commands::LogLevel;

const BAR_LEN: usize = 50;

pub struct ProgressInfo {
    len: usize,
    current: usize,
    title: String,
}

pub struct CliReporter {
    stdout: BufWriter<StdoutLock<'static>>,
    progress: Option<ProgressInfo>,
    log_level: LogLevel,
}

impl CliReporter {
    pub fn new(log_level: LogLevel) -> Self {
        Self {
            stdout: BufWriter::with_capacity(10, stdout().lock()),
            progress: None,
            log_level,
        }
    }
}

impl ProgressReport for CliReporter {
    fn begin_progress(&mut self, title: String, len: usize) {
        if self.log_level != LogLevel::All {
            return;
        }

        write!(&mut self.stdout, "\u{001B}[?25l").expect("Can't write into stdout"); // hide console cursor
        self.progress = Some(ProgressInfo { len, current: 0, title });
    }

    fn add_progress(&mut self) {
        if let Some(ProgressInfo { len, current, title }) = self.progress.as_mut() {
            *current += 1;

            let filled_up_length = BAR_LEN * *current / *len;
            let percentage = 100 * *current / *len;

            write!(
                &mut self.stdout,
                "\r[P]: {title} [{}{}] {percentage:03}% ({current}/{len})",
                "#".repeat(filled_up_length),
                "-".repeat(BAR_LEN - filled_up_length),
            )
            .expect("Can't write into stdout");
        }
    }

    fn end_progress(&mut self) {
        if self.progress.take().is_some() {
            writeln!(&mut self.stdout, "\u{001B}[?25h").expect("Can't write into stdout"); // show console cursor + newline
            self.stdout.flush().expect("Can't flush stdout");
        }
    }
}

impl Logger for CliReporter {
    fn info(&mut self, str: impl AsRef<str>) {
        if !matches!(self.log_level, LogLevel::All | LogLevel::NoProgress) {
            return;
        }

        writeln!(&mut self.stdout, "[?]: {}", str.as_ref()).expect("Can't write into stdout");
        self.stdout.flush().expect("Can't flush stdout");
    }

    fn good(&mut self, str: impl AsRef<str>) {
        if !matches!(self.log_level, LogLevel::All | LogLevel::NoProgress) {
            return;
        }

        writeln!(&mut self.stdout, "[+]: {}", str.as_ref()).expect("Can't write into stdout");
        self.stdout.flush().expect("Can't flush stdout");
    }

    fn warn(&mut self, str: impl AsRef<str>) {
        if matches!(self.log_level, LogLevel::Error | LogLevel::Nothing) {
            return;
        }

        writeln!(&mut self.stdout, "[!]: {}", str.as_ref()).expect("Can't write into stdout");
        self.stdout.flush().expect("Can't flush stdout");
    }

    fn error(&mut self, str: impl AsRef<str>) {
        if self.log_level == LogLevel::NoProgress {
            return;
        }

        writeln!(&mut self.stdout, "[-]: {}", str.as_ref()).expect("Can't write into stdout");
        self.stdout.flush().expect("Can't flush stdout");
    }
}

impl Drop for CliReporter {
    fn drop(&mut self) {
        if self.progress.is_some() {
            self.end_progress();
        } else {
            self.stdout.flush().expect("Can't flush stdout");
        }
    }
}
