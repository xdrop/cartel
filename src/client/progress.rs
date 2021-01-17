use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Clone)]
pub struct SpinnerOptions {
    pub style: ProgressStyle,
    pub message: String,
    pub step: Option<(u64, u64)>,
    pub clear_on_finish: Option<bool>,
}

pub enum SpinnerCmd {
    ExitWithStatus { status: String },
    Exit,
    ExitAndClear,
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
    ///     .step(1,4)
    ///     .clear_on_finish(true);
    /// let wu = WaitUntil::new(&spin_opts);
    /// ```
    pub fn new(message: String) -> SpinnerOptions {
        SpinnerOptions {
            style: ProgressStyle::default_spinner()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                .template("{prefix:.bold.dim} {spinner} {wide_msg}"),
            message: format!("  {}", message),
            step: None,
            clear_on_finish: None,
        }
    }

    /// Sets the step that this operation is on for.
    pub fn step(mut self, step: u64, of: u64) -> SpinnerOptions {
        self.step = Some((step, of));
        self
    }

    /// Sets the clear on finish flag. When set to true the spinner line will be
    /// erased from the terminal.
    pub fn clear_on_finish(mut self, clear_on_finish: bool) -> SpinnerOptions {
        self.clear_on_finish = Some(clear_on_finish);
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
    control: Option<mpsc::Sender<SpinnerCmd>>,
    handle: Option<std::thread::JoinHandle<()>>,
    clear_on_finish: Option<bool>,
}

impl<'a> WaitSpin<'a> {
    pub fn new(options: &'a SpinnerOptions) -> WaitSpin {
        WaitSpin {
            options,
            control: None,
            handle: None,
            clear_on_finish: None,
        }
    }

    /// Start the spinner.
    ///
    /// This method spins up a new thread to render the spinner on the screen.
    /// The spinner will keep spinning until `stop` is called.
    pub fn start(&mut self) {
        let options = self.options.clone();
        let (tx, rx) = mpsc::channel::<SpinnerCmd>();

        self.control = Some(tx);
        self.handle = Some(std::thread::spawn(move || {
            let pb = ProgressBar::new(std::u64::MAX);
            pb.set_style(options.style);
            if let Some((step, of)) = options.step {
                pb.set_prefix(&format!("[{}/{}]  ", step, of));
            } else {
                pb.set_prefix("     ");
            };
            pb.set_message(&options.message);

            loop {
                match rx.try_recv() {
                    Ok(SpinnerCmd::ExitWithStatus { status }) => {
                        pb.set_message(&format!(
                            "{} {}",
                            options.message, status
                        ));
                        pb.finish();
                        break;
                    }
                    Ok(SpinnerCmd::ExitAndClear) => {
                        pb.finish_and_clear();
                        break;
                    }
                    Ok(SpinnerCmd::Exit) => {
                        pb.finish();
                        break;
                    }
                    Err(_) => {
                        pb.inc(1);
                        std::thread::sleep(Duration::from_millis(100));
                    }
                };
            }
        }));
    }

    /// Stops the spinner.
    pub fn stop(self) {
        if let Some(control) = self.control {
            if let Some(handle) = self.handle {
                control
                    .send(SpinnerCmd::Exit)
                    .expect("Unexpected failure in WaitSpin::stop");
                handle.join().expect("CLI thread failed");
            }
        }
    }

    /// Stops the spinner and clears the last line.
    pub fn stop_and_clear(self) {
        if let Some(control) = self.control {
            if let Some(handle) = self.handle {
                control
                    .send(SpinnerCmd::ExitAndClear)
                    .expect("Unexpected failure in WaitSpin::stop_and_clear");
                handle.join().expect("CLI thread failed");
            }
        }
    }

    /// Stops the spinner and updates the status of the last line.
    pub fn stop_with_status(self, status: String) {
        if let Some(control) = self.control {
            if let Some(handle) = self.handle {
                control
                    .send(SpinnerCmd::ExitWithStatus { status })
                    .expect("Unexpected failure in WaitSpin::stop_with_status");
                handle.join().expect("CLI thread failed");
            }
        }
    }
}

/// A utility to render CLI spinners until some operation is complete.
pub struct WaitUntil<'a> {
    options: &'a SpinnerOptions,
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
    pub fn new(options: &'a SpinnerOptions) -> WaitUntil {
        WaitUntil { options }
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
        let mut ws = WaitSpin::new(self.options);
        ws.start();
        let fn_res = f();
        ws.stop();
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
    pub fn spin_until_status<F, T>(&mut self, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<WaitResult<T>>,
    {
        let mut ws = WaitSpin::new(self.options);
        ws.start();
        let wait_result = f();
        match wait_result {
            Ok(w) => {
                ws.stop_with_status(w.status);
                Ok(w.result)
            }
            Err(e) => {
                ws.stop();
                Err(e)
            }
        }
    }
}
