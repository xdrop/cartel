pub use self::imp::Process;

#[cfg(target_family = "unix")]
mod imp {
    use nix::errno::Errno;
    use nix::libc::c_int;
    use nix::sys::wait::{WaitPidFlag, WaitStatus};
    use nix::unistd::*;
    use nix::{self, libc, Error};
    use std::collections::HashMap;
    use std::convert::TryInto;
    use std::fs::File;
    use std::io::{self, Result};
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::path::Path;
    use std::process::{Child, Command, ExitStatus, Stdio};

    #[derive(Debug)]
    pub struct Process {
        /// The group id of the process.
        pgid: libc::pid_t,
        child: Child,
    }

    impl Process {
        /// Executes the command as a child process, returning a handle to it.
        ///
        /// Unlike [std::Process], this (Unix specific) implementation will use
        /// the `setsid` syscall to dissacociate the child process from the
        /// parents pgid, allowing to signal the process group of the child
        /// independently.
        ///
        /// All operations on the returned handle happen on the `pgid` rather
        /// than the `pid` of the child.
        ///
        /// # Arguments
        ///
        /// * `cmd` - The command to execute. Must have length of at least one.
        /// * `env` - The environment variables to create the new process with.
        /// * `stdout` - A [File] where stdout is written to.
        /// * `stderr` - A [File] where stderr is written to.
        /// * `work_dir` - The working directory the process will run in.
        pub fn spawn(
            cmd: &[String],
            env: &HashMap<String, String>,
            stdout: File,
            stderr: File,
            work_dir: Option<&Path>,
        ) -> Result<Self> {
            let (head, tail) =
                cmd.split_first().expect("Empty command in Process::spawn");

            let mut command = Command::new(head);
            command
                .args(tail)
                .envs(env)
                .stdout(Stdio::from(stdout))
                .stderr(Stdio::from(stderr));

            if let Some(path) = work_dir {
                command.current_dir(path);
            }

            unsafe {
                // Create a new session for the process by calling `setsid`. The
                // process group ID and session ID of the calling process are
                // set to the PID of the calling process (and thus distinct from
                // the daemon's pgid).
                command
                    .pre_exec(|| setsid().map_err(from_nix_error).map(|_| ()));
            }

            command.spawn().map(|p| {
                let id = p.id();
                Self {
                    child: p,
                    pgid: id
                        .try_into() // pgid is negative
                        .expect("u32 -> i32 failed in Process::spawn"),
                }
            })
        }

        /// Sends SIGINT to the pgid of this process.
        pub fn interrupt(&mut self) {
            self.signal_process_group(libc::SIGINT);
        }

        /// Sends SIGTERM to the pgid of this process.
        pub fn terminate(&mut self) {
            self.signal_process_group(libc::SIGTERM);
        }

        /// Sends SIGKILL to the pgid of this process.
        pub fn kill(&mut self) {
            self.signal_process_group(libc::SIGKILL);
        }

        /// Wait for the process group to exit.
        ///
        /// This function blocks the calling thread and will only finish once
        /// the child has exited.
        pub fn wait(&mut self) -> nix::Result<ExitStatus> {
            loop {
                match waitpid(
                    Pid::from_raw(-self.pgid),
                    Some(WaitPidFlag::WNOHANG),
                ) {
                    Ok((WaitStatus::Exited(_, _), status)) => {
                        return Ok(ExitStatus::from_raw(status))
                    }
                    Ok((WaitStatus::Signaled(_, _, _), status)) => {
                        return Ok(ExitStatus::from_raw(status))
                    }
                    Ok(_) => {} // continue
                    Err(e) => return Err(e),
                }
            }
        }

        /// Return the pid of the child process.
        #[inline]
        pub fn id(&self) -> u32 {
            self.child.id()
        }

        /// Attempts to collect the exit status of the child if it has already exited.
        ///
        /// This function will not block the calling thread and will only check
        /// to see if the child process has exited or not.
        #[inline]
        pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            self.child.try_wait()
        }

        /// Sends a signal to the process group using [libc::killpg]
        fn signal_process_group(&self, sig: libc::c_int) {
            extern "C" {
                fn killpg(pgrp: libc::pid_t, sig: libc::c_int) -> libc::c_int;
            }

            unsafe {
                killpg(self.pgid, sig);
            }
        }
    }

    /// Wrapper for `waitpid` libc syscall that additionally returns the raw
    /// wait status.
    ///
    /// This is identical to [`nix::sys::wait::waitpid`] expect it also
    /// includes the raw wait status `i32` as the second tuple argument.
    fn waitpid<P: Into<Option<Pid>>>(
        pid: P,
        options: Option<WaitPidFlag>,
    ) -> nix::Result<(WaitStatus, i32)> {
        let mut status: i32 = 0;
        let option_bits = match options {
            Some(bits) => bits.bits(),
            None => 0,
        };

        let res = unsafe {
            libc::waitpid(
                pid.into().unwrap_or_else(|| Pid::from_raw(-1)).into(),
                &mut status as *mut c_int,
                option_bits,
            )
        };

        match Errno::result(res)? {
            0 => Ok((WaitStatus::StillAlive, status)),
            res => WaitStatus::from_raw(Pid::from_raw(res), status)
                .map(|ws| (ws, status)),
        }
    }

    /// Convert a *nix error into io:Error.
    fn from_nix_error(err: nix::Error) -> io::Error {
        match err {
            Error::Sys(errno) => io::Error::from_raw_os_error(errno as i32),
            Error::InvalidPath => {
                io::Error::new(io::ErrorKind::InvalidInput, err)
            }
            _ => io::Error::new(io::ErrorKind::Other, err),
        }
    }
}

#[cfg(target_family = "windows")]
mod imp {
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{self, ExitStatus, Result};
    use std::path::Path;
    use std::process::{Child, Command, Stdio};

    #[derive(Debug)]
    pub struct Process {
        pub child: Child,
    }

    /// The Windows implementation is extremely poor and will not do the "right"
    /// thing when it comes to killing a service (the service's subprocesses may
    /// continue to live). This requires an imlementation using JobObjects from
    /// the Windows API.
    impl Process {
        pub fn spawn(
            cmd: &[String],
            env: &HashMap<String, String>,
            stdout: File,
            stderr: File,
            work_dir: Option<&Path>,
        ) -> Result<Self> {
            let (head, tail) =
                cmd.split_first().expect("Empty command in Process::spawn");

            let mut command = Command::new(head);
            command
                .args(tail)
                .envs(env)
                .stdout(Stdio::from(stdout))
                .stderr(Stdio::from(stderr));

            if let Some(path) = work_dir {
                command.current_dir(path);
            }

            command.spawn()
        }

        #[inline]
        pub fn terminate(&mut self) {
            self.child.kill()
        }

        #[inline]
        pub fn interrupt(&mut self) {
            self.child.kill()
        }

        #[inline]
        pub fn kill(&mut self) {
            self.child.kill()
        }

        #[inline]
        pub fn id(&self) -> u32 {
            self.child.id()
        }

        #[inline]
        pub fn wait(&mut self) -> Result<ExitStatus> {
            self.child.wait()
        }

        #[inline]
        pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            self.child.try_wait()
        }
    }
}
