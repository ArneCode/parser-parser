use crate::grammar::label::MaybeLabel;

pub trait ErrorHandler: Default {
    type Indexer;

    fn register_start(&mut self) -> Self::Indexer;
    fn register_error<L: Display + 'static, O: MaybeLabel<Label = L>>(
        &mut self,
        obj: &O,
        idx: Self::Indexer,
        match_start: usize,
        failure_slice: (usize, usize),
    );
}
#[derive(Default)]
pub struct EmptyErrorHandler {}

impl ErrorHandler for EmptyErrorHandler {
    type Indexer = ();

    fn register_start(&mut self) -> Self::Indexer {}
    fn register_error<L: Display, O: MaybeLabel<Label = L>>(
        &mut self,
        _obj: &O,
        _idx: Self::Indexer,
        _match_start: usize,
        _failure_slice: (usize, usize),
    ) {
    }
}

pub struct ErrorDescription<L: ?Sized> {
    label: Box<L>,
}
pub struct MultiErrorHandler {
    best_failure_slice: (usize, usize),
    errors: Vec<Option<ErrorDescription<dyn Display>>>, // indexed by the indexer returned by register_start
    errors_at_match_start: Vec<usize>, // indices of errors that occurred at their match_start
}

impl Default for MultiErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiErrorHandler {
    pub fn new() -> Self {
        Self {
            best_failure_slice: (0, 0),
            errors: Vec::new(),
            errors_at_match_start: Vec::new(),
        }
    }
}

impl ErrorHandler for MultiErrorHandler {
    type Indexer = usize;

    fn register_start(&mut self) -> Self::Indexer {
        self.errors.push(None);
        self.errors.len() - 1
    }

    fn register_error<L: Display + 'static, O: MaybeLabel<Label = L>>(
        &mut self,
        obj: &O,
        mut idx: Self::Indexer,
        match_start: usize,
        failure_slice: (usize, usize),
    ) {
        // check if this error is less interesting than the best failure so far
        if failure_slice.1 < self.best_failure_slice.1
            || failure_slice.0 < self.best_failure_slice.0
        {
            // This error is worse, so ignore it.
            return;
        }
        let label = if let Some(label) = obj.maybe_label() {
            label
        } else {
            // If the object doesn't have a label, we can't provide a useful error message, so ignore it.
            return;
        };
        // check if this error is more interesting than the best failure so far
        if failure_slice.1 > self.best_failure_slice.1
            || failure_slice.0 > self.best_failure_slice.0
        {
            // This error is better, so clear the previous errors and update the best failure slice.
            self.errors.clear();
            self.errors_at_match_start.clear();
            self.best_failure_slice = failure_slice;
            self.errors.push(None); // placeholder for this error
            idx = self.errors.len() - 1; // update idx to point to the new error
        }

        // remove all errors that occur inside this rule that haven't matched a single token
        while !self.errors_at_match_start.is_empty()
            && self.errors_at_match_start.last().unwrap() > &idx
        {
            self.errors_at_match_start.pop();
        }

        // Now this error is at least as interesting as the best failure, so register it.
        self.errors[idx] = Some(ErrorDescription {
            label: Box::new(label),
        });

        // If this error occurs at its match_start, add it to the list of errors at match_start.
        if match_start == failure_slice.0 {
            self.errors_at_match_start.push(idx);
        }
    }
}

use ariadne::{Color, Label, Report, ReportKind, Source};
use std::collections::HashSet;
use std::fmt::Display;

impl MultiErrorHandler {
    pub fn render_report(&self, source_id: &str, source_text: &str) {
        // 1. Collect all unique expected labels
        let expected_labels: Vec<String> = self
            .errors
            .iter()
            .flatten() // Remove Nones
            .map(|err| format!("{}", err.label))
            .collect::<HashSet<_>>() // Deduplicate
            .into_iter()
            .collect();

        let expected_str = if expected_labels.is_empty() {
            "something else".to_string()
        } else {
            expected_labels.join(", ")
        };

        // 2. Identify what was actually "found" at the failure slice
        // We look at the start of the best_failure_slice
        let found = source_text
            .get(self.best_failure_slice.0..self.best_failure_slice.1)
            .unwrap_or({
                // Handle EOF (End of File)
                if self.best_failure_slice.0 >= source_text.len() {
                    "end of input"
                } else {
                    "unknown token"
                }
            });

        // 3. Build the main diagnostic message
        let main_message = format!("expected one of {} but found '{}'", expected_str, found);

        // 4. Create the Ariadne Report
        let report = Report::build(
            ReportKind::Error,
            (
                source_id,
                self.best_failure_slice.0..self.best_failure_slice.1,
            ),
        )
        .with_message("Syntax Error")
        .with_label(
            Label::new((
                source_id,
                self.best_failure_slice.0..self.best_failure_slice.1,
            ))
            .with_message(main_message)
            .with_color(Color::Red),
        );

        // 5. (Optional) Add labels for the "Why" (the rule stack)
        // for (i, error_opt) in self.errors.iter().enumerate() {
        //     if let Some(desc) = error_opt {
        //         report = report.with_label(
        //             Label::new((source_id, desc.match_start..self.best_failure_slice.1))
        //                 .with_message(format!("while attempting to parse {}", desc.label))
        //                 .with_color(Color::Cyan),
        //         );
        //     }
        // }

        report
            .finish()
            .eprint((source_id, Source::from(source_text.to_string())))
            .unwrap();
    }
}
