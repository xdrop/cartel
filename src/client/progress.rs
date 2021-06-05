use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Clone)]
pub struct SpinnerOptions {
    pub style: ProgressStyle,
    pub message: String,
    pub failure_message: String,
    pub step: Option<(u64, u64)>,
}

impl SpinnerOptions {
    /// Creates a `SpinnerOptions` object to represent settings for the spinner.
    ///
    /// # Examples
    ///
    /// This renders a line like `[1/4]  Waiting...` and hides it on completion.
    /// ```
    /// use cartel::client::progress::*;
    /// let spin_opts = SpinnerOptions::new(String::from("Waiting..."))
    ///     .step(1,4);
    /// let wu = WaitUntil::new(&spin_opts);
    /// ```
    pub fn new(message: String) -> SpinnerOptions {
        SpinnerOptions {
            style: ProgressStyle::default_spinner()
                .template("{prefix:.bold.dim} {spinner} {wide_msg}"),
            message: format!("  {}", message),
            failure_message: console::style("(Failed)")
                .red()
                .bold()
                .to_string(),
            step: None,
        }
    }

    /// Sets the step that this operation is on for.
    pub fn step(mut self, step: u64, of: u64) -> SpinnerOptions {
        self.step = Some((step, of));
        self
    }

    /// Set the failure message.
    pub fn failure_msg(mut self, failure_message: String) -> SpinnerOptions {
        self.failure_message = failure_message;
        self
    }

    /// Set the progress bar style.
    pub fn style(mut self, style: ProgressStyle) -> SpinnerOptions {
        self.style = style;
        self
    }
}
/// Creates and renders a 'wait' spinner.
///
/// To be used for CLI interactions while waiting for an operation to finish.
///
/// # Examples
///
/// ```ignore
/// let spin_opts = SpinnerOptions::new(String::from("Waiting..."));
/// let ws = WaitSpin::new(&spin_opts);
/// ws.start();
/// // Perform operation x
/// ws.stop();
/// ```
pub struct WaitSpin<'a> {
    options: &'a SpinnerOptions,
    pb: ProgressBar,
}

impl<'a> WaitSpin<'a> {
    pub fn new(options: &'a SpinnerOptions) -> WaitSpin {
        WaitSpin {
            options,
            pb: ProgressBar::new(std::u64::MAX),
        }
    }

    pub fn from(options: &'a SpinnerOptions, pb: ProgressBar) -> WaitSpin {
        WaitSpin { options, pb }
    }

    /// Start the spinner.
    ///
    /// The spinner will keep spinning until `stop` is called.
    pub fn start(&mut self) {
        let options = self.options.clone();

        self.pb.set_style(options.style);
        if let Some((step, of)) = options.step {
            self.pb.set_prefix(format!("[{}/{}]  ", step, of));
        } else {
            self.pb.set_prefix("     ");
        };
        self.pb.set_message(options.message);
        self.pb.enable_steady_tick(100);
    }

    /// Stops the spinner.
    pub fn stop(&self) {
        self.pb.finish();
    }

    /// Stops the spinner and clears the last line.
    pub fn stop_and_clear(&self) {
        self.pb.finish_and_clear();
    }

    /// Stops the spinner and updates the status of the last line.
    pub fn stop_with_status(&mut self, status: String) {
        self.pb.finish_with_message(format!(
            "{} {}",
            self.options.message, status
        ));
    }

    /// Stops the spinner and sets the status to error.
    pub fn stop_with_error(&mut self) {
        let status = self.options.failure_message.clone();
        self.pb.finish_with_message(format!(
            "{} {}",
            self.options.message, status
        ));
    }
}

/// A utility to render CLI spinners until some operation is complete.
pub struct WaitUntil<'a> {
    wait_spin: WaitSpin<'a>,
}

pub struct WaitResult<T> {
    result: T,
    status: String,
}

impl<T> WaitResult<T> {
    pub fn from(result: T, status: String) -> WaitResult<T> {
        WaitResult { result, status }
    }
}

impl<'a> WaitUntil<'a> {
    /// Creates a new `WaitUntil` that can be used to render a wait spinner
    /// until an operation is complete.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let spin_opts = SpinnerOptions::new(String::from("Waiting..."));
    /// let wu = WaitUnti::new(&spin_opts);
    /// wu.spin_until(|| {
    ///     std::thread::sleep(5000);
    /// });
    /// ```
    pub fn new_multi(
        options: &'a SpinnerOptions,
        progress_bar: ProgressBar,
    ) -> WaitUntil {
        WaitUntil {
            wait_spin: WaitSpin::from(options, progress_bar),
        }
    }

    pub fn new(options: &'a SpinnerOptions) -> WaitUntil {
        WaitUntil {
            wait_spin: WaitSpin::from(options, ProgressBar::new(std::u64::MAX)),
        }
    }

    /// Renders a spinner until the closure completes.
    ///
    /// Note that the closure must be free of CLI side effects. Things like
    /// calls to `println!` during the closure's operation may lead to undefined
    /// behaviour.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let spin_opts = SpinnerOptions::new(String::from("Waiting..."));
    /// let wu = WaitUnti::new(&spin_opts);
    /// wu.spin_until(|| {
    ///     std::thread::sleep(5000);
    /// });
    /// ```
    pub fn spin_until<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.wait_spin.start();
        let fn_res = f();
        self.wait_spin.stop();
        fn_res
    }

    /// Renders a spinner until the closure completes and updates the status.
    ///
    /// This, unlike [spin_until], will also update the line status on
    /// completion. The status message to render on the line has to be provided
    /// by the closure by wrapping the result in a `WaitResult<T>`.
    ///
    /// Note that the closure must be free of CLI side effects. Things like
    /// calls to `println!` during the closure's operation may lead to undefined
    /// behaviour.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let spin_opts = SpinnerOptions::new(String::from("Waiting..."));
    /// let wu = WaitUnti::new(&spin_opts);
    /// wu.spin_until_status(|| {
    ///     std::thread::sleep(5000);
    ///     Ok(WaitResult::from((), String::from("(Done)")))
    /// });
    /// ```
    pub fn spin_until_status<F, T>(mut self, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<WaitResult<T>>,
    {
        self.wait_spin.start();
        let wait_result = f();
        match wait_result {
            Ok(w) => {
                self.wait_spin.stop_with_status(w.status);
                Ok(w.result)
            }
            Err(e) => {
                self.wait_spin.stop_with_error();
                Err(e)
            }
        }
    }
}
