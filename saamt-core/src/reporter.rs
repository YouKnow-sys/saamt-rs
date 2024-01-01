//! The main reporter module, it can be used send back progress and logs to user.

/// The ProgressReport trait defines methods for reporting progress
/// during long running operations. This can be implemented to provide
/// visual feedback in a UI or log output.
pub trait ProgressReport: Sized {
    /// start a progress with the given len and title.
    fn begin_progress(&mut self, title: String, len: usize);
    /// add one to the progress.
    fn add_progress(&mut self);
    /// end the progress.
    fn end_progress(&mut self);
}

/// The Logger trait defines logging methods that can be implemented to handle
/// log messages from the core library. This is a public API that can be used by
/// external consumers of the library to receive log output.
pub trait Logger: Sized {
    /// Log a **info** message
    fn info(&mut self, str: impl AsRef<str>);
    /// Log a message
    fn good(&mut self, str: impl AsRef<str>);
    /// Log a **warning** message
    fn warn(&mut self, str: impl AsRef<str>);
    /// Log a **error** message
    fn error(&mut self, str: impl AsRef<str>);
}

/// Provides methods to wrap an `Iterator` in a `ProgressReporterIter`
/// to report progress on each iteration easier.
pub trait ProgressReporterIterator: Iterator + Sized {
    /// Wraps an iterator in a `ProgressReporterIter` struct to report progress on each iteration to the provided `ProgressReport`.
    ///
    /// Begins a new progress report with the given title and length. Returns a `ProgressReporterIter`
    /// that updates the progress on each iteration of the underlying iterator.
    fn progress_report<P: ProgressReport>(
        self,
        reporter: &mut P,
        len: usize,
        title: String,
    ) -> ProgressReporterIter<P, Self> {
        reporter.begin_progress(title, len);

        ProgressReporterIter {
            reporter,
            underlying: self,
        }
    }
}

/// ProgressReporterIter is a struct that wraps an Iterator
/// and updates a ProgressReport on each iteration. It is used to
/// provide progress feedback when iterating over a collection.
pub struct ProgressReporterIter<'a, P, I>
where
    P: ProgressReport,
    I: Iterator,
{
    reporter: &'a mut P,
    underlying: I,
}

impl<'a, P, I> Iterator for ProgressReporterIter<'a, P, I>
where
    I: Iterator,
    P: ProgressReport,
{
    type Item = <I as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.underlying.next();

        match item.is_some() {
            true => self.reporter.add_progress(),
            false => self.reporter.end_progress(),
        }

        item
    }
}

impl<I: Iterator> ProgressReporterIterator for I {}
