use crate::thread_control::{make_pair, Control, Flag};
use indicatif::{ProgressBar, ProgressStyle};

/// Creates and renders a 'wait' spinner.
///
/// To be used for CLI interactions while waiting for an operation to finish.
///
/// # Examples
///
/// ```ignore
/// let ws = WaitSpin::new();
/// ws.start(1,3,"Waiting for x...");
/// // Perform operation x
/// ws.stop();
/// ```
pub struct WaitSpin {
    spinner_style: ProgressStyle,
    control: Option<Control>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl WaitSpin {
    pub fn new() -> WaitSpin {
        WaitSpin {
            spinner_style: ProgressStyle::default_spinner()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                .template("{prefix:.bold.dim} {spinner} {wide_msg}"),
            control: None,
            handle: None,
        }
    }

    /// Start the spinner.
    ///
    /// This method spins up a new thread to render the spinner on the screen.
    /// The spinner will keep spinning until `stop` is called.
    ///
    /// # Arguments
    ///
    /// * `step` - The current step we are on
    /// * `of` - The total number of steps
    /// * `message` - The message to print while waiting (and after)
    pub fn start(&mut self, step: u64, of: u64, message: String) -> () {
        let style = self.spinner_style.clone();
        let (flag, control) = make_pair();
        self.control = Some(control);
        self.handle = Some(std::thread::spawn(move || {
            let pb = ProgressBar::new(std::u64::MAX);
            pb.set_style(style);
            pb.set_prefix(&format!("[{}/{}]", step, of));
            pb.set_message(&message);

            while flag.alive() {
                pb.inc(1);
                std::thread::sleep_ms(100);
            }
            pb.finish();
        }));
    }

    /// Stops the spinner.
    pub fn stop(self) -> () {
        if let Some(control) = self.control {
            if let Some(handle) = self.handle {
                control.stop();
                handle.join().expect("CLI thread failed");
            }
        }
    }
}

/// A utility to render CLI spinners until some operation is complete.
pub struct WaitUntil {
    step: u64,
    of: u64,
    message: String,
}

impl WaitUntil {
    /// Creates a new `WaitUntil` that can be used to render a wait spinner
    /// until an operation is complete.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let wu = WaitUnti::new(1, 3, "Deploying...");
    /// wu.spin_until(|| {
    ///     std::thread::sleep(5000);
    /// });
    /// ```
    pub fn new(step: u64, of: u64, message: String) -> WaitUntil {
        WaitUntil { step, of, message }
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
    /// let wu = WaitUnti::new(1, 3, "Deploying...");
    /// wu.spin_until(|| {
    ///     std::thread::sleep(5000);
    /// });
    /// ```
    pub fn spin_until<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let mut ws = WaitSpin::new();
        ws.start(self.step, self.of, self.message.clone());
        let fn_res = f();
        ws.stop();
        fn_res
    }
}
