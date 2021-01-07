pub use self::implementation::Process;

#[cfg(target_family = "unix")]
mod implementation {
    use nix::libc;
    use nix::{self, Error};
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{self, Result};
    use std::path::Path;
    use std::process::{Child, Command, ExitStatus, Stdio};

    #[derive(Debug)]
    pub struct Process {
        /// The group id of the process.
        pgid: libc::pid_t,
        child: Child,
    }

    impl Process {
        pub fn spawn(
            cmd: &[String],
            env: &HashMap<String, String>,
            stdout: File,
            stderr: File,
            work_dir: Option<&Path>,
        ) -> Result<Self> {
            use nix::unistd::*;
            use std::convert::TryInto;
            use std::os::unix::process::CommandExt;

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
            command.spawn().and_then(|p| {
                let id = p.id();
                Ok(Self {
                    child: p,
                    pgid: id
                        .try_into()
                        .expect("u32 -> i32 failed in Process::spawn"),
                })
            })
        }

        pub fn interrupt(&mut self) -> () {
            self.signal_process_group(libc::SIGINT);
        }

        pub fn terminate(&mut self) -> () {
            self.signal_process_group(libc::SIGTERM);
        }

        pub fn kill(&mut self) -> () {
            self.signal_process_group(libc::SIGKILL);
        }

        pub fn wait(&mut self) -> () {
            use nix::sys::wait::*;
            use nix::unistd::Pid;

            loop {
                match waitpid(
                    Pid::from_raw(-self.pgid),
                    Some(WaitPidFlag::WNOHANG),
                ) {
                    Ok(WaitStatus::Exited(_, _)) => break,
                    Ok(WaitStatus::Signaled(_, _, _)) => break,
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }

        #[inline]
        pub fn id(&self) -> u32 {
            self.child.id()
        }

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
mod implementation {
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{self, Result};
    use std::path::Path;
    use std::process::{Child, Command, Stdio};

    #[derive(Debug)]
    pub struct Process {
        pub child: Child,
    }

    /// The Windows implementation is extremely poor and will not do the "right"
    /// thing when it comes to killing a service (the service's subprocesses may
    /// continue to live). This requires an imlementation using JobObjects from
    /// the Windows API
    impl Process {
        pub fn spawn(
            cmd: &[String],
            env: &HashMap<String, String>,
            stdout: File,
            stderr: File,
            work_dir: Option<&Path>,
        ) -> Result<Self> {
            use nix::unistd::*;
            use std::convert::TryInto;
            use std::os::unix::process::CommandExt;

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
        pub fn terminate(&mut self) -> () {
            self.child.kill()
        }

        #[inline]
        pub fn interrupt(&mut self) -> () {
            self.child.kill()
        }

        #[inline]
        pub fn kill(&mut self) -> () {
            self.child.kill()
        }

        #[inline]
        pub fn id(&self) -> u32 {
            self.child.id()
        }

        #[inline]
        pub fn wait(&mut self) -> () {
            self.child.wait()
        }

        #[inline]
        pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            self.child.try_wait()
        }
    }
}