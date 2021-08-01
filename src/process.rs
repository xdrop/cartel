use std::io::Result;
use std::process::{Child, ExitStatus};

#[derive(Debug)]
pub struct GroupChild {
    imp: imp::GroupChildImp,
}

#[derive(Debug)]
pub enum Process {
    Ungroupped(Child),
    Groupped(GroupChild),
}

/// Representation of either a groupped or ungroupped process.
///
/// This enum is used to manage processes that can be either groupped or
/// ungroupped, delegating calls to the appropriate implementation.
///
/// Groupped processes are created via [`Command#group_spawn`], while ungroupped
/// processes are created via [`Command#spawn`].
impl Process {
    /// Wrap the given `Child` to represent an ungroupped process.
    pub fn ungroupped(child: Child) -> Process {
        Process::Ungroupped(child)
    }

    /// Wrap the given `GroupChild` to represent a groupped process.
    pub fn groupped(child: GroupChild) -> Process {
        Process::Groupped(child)
    }

    /// Interrupt the process.
    ///
    /// On Unix this sends `SIGINT` to the process (if ungroupped) or process
    /// group (if groupped). On Windows this will always perform a
    /// [`std::process::Child#kill`].
    pub fn interrupt(&mut self) -> Result<()> {
        match self {
            Self::Groupped(grp) => grp.interrupt(),
            Self::Ungroupped(ungrp) => ungrp.interrupt(),
        }
    }

    /// Terminate the process.
    ///
    /// On Unix this sends `SIGTERM` to the process (if ungroupped) or process
    /// group (if groupped). On Windows this will always perform a
    /// [`std::process::Child#kill`].
    pub fn terminate(&mut self) -> Result<()> {
        match self {
            Self::Groupped(grp) => grp.terminate(),
            Self::Ungroupped(ungrp) => ungrp.terminate(),
        }
    }

    /// Kill the process.
    ///
    /// On Unix this sends `SIGKILL` to the process (if ungroupped) or process
    /// group (if groupped). On Windows this will always perform a
    /// [`std::process::Child#kill`].
    pub fn kill(&mut self) -> Result<()> {
        match self {
            Self::Groupped(grp) => grp.kill(),
            Self::Ungroupped(ungrp) => ungrp.kill(),
        }
    }

    /// Return the process id.
    ///
    /// On Unix this will be the `pid` of the process (if ungroupped) or the
    /// pgid of the group (if groupped). On Windows the default implementation
    /// of [`std::process:Child#id`] is used.
    pub fn id(&self) -> u32 {
        match self {
            Self::Groupped(grp) => grp.id(),
            Self::Ungroupped(ungrp) => ungrp.id(),
        }
    }

    /// Waits for the process (or group) to exit completely, returning the
    /// status that it exited with.
    pub fn wait(&mut self) -> Result<ExitStatus> {
        match self {
            Self::Groupped(grp) => grp.wait(),
            Self::Ungroupped(ungrp) => ungrp.wait(),
        }
    }

    /// Attempts to collect the exit status of the process (or group) if it has
    /// already exited.
    ///
    /// This function will not block the calling thread and will only check to
    /// see if the child process has exited or not.
    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        match self {
            Self::Groupped(grp) => grp.try_wait(),
            Self::Ungroupped(ungrp) => ungrp.try_wait(),
        }
    }
}

pub trait CommandExt {
    /// Executes the command as a child process in a new group, returning a
    /// handle to it.
    ///
    /// On Unix, the implementation will use the `setsid` syscall to create a
    /// new session for the process. The process will become the leader of the
    /// new session, and a new process group will be created in this session.
    /// This process will become the process group leader of the new process
    /// group in the session.
    ///
    /// Any signals sent, as well as any wait operations will be performed on
    /// this newly created process group (which has a pgid equivalent to the pid
    /// of the leader).
    ///
    /// On Windows this is currently not implemented and will default to using
    /// the regular stdlib `Child` operations.
    fn group_spawn(&mut self) -> Result<GroupChild>;
}

pub trait ChildExt {
    /// Interrupt the child process.
    ///
    /// On Unix this sends `SIGKILL` to the pid of this process. On Windows this
    /// will perform a [`std::process::Child#kill`].
    fn interrupt(&mut self) -> Result<()>;

    /// Terminate the child process.
    ///
    /// On Unix this sends `SIGTERM` to the pgid of this process. On Windows this
    /// will perform a [`std::process::Child#kill`].
    fn terminate(&mut self) -> Result<()>;

    /// Kill the child proces .
    ///
    /// On Unix this sends `SIGKILL` to the pgid of this process. On Windows this
    /// will perform a [`std::process::Child#kill`].
    fn kill(&mut self) -> Result<()>;
}

impl GroupChild {
    /// Interrupt the child process group.
    ///
    /// On Unix this sends `SIGINT` to the pgid of this process. On Windows this
    /// will perform a [`std::process::Child#kill`].
    pub fn interrupt(&mut self) -> Result<()> {
        self.imp.interrupt()
    }

    /// Terminate the child process group.
    ///
    /// On Unix this sends `SIGTERM` to the pgid of this process. On Windows this
    /// will perform a [`std::process::Child#kill`].
    pub fn terminate(&mut self) -> Result<()> {
        self.imp.terminate()
    }

    /// Kill the child process group.
    ///
    /// On Unix this sends `SIGKILL` to the pgid of this process. On Windows this
    /// will perform a [`std::process::Child#kill`].
    pub fn kill(&mut self) -> Result<()> {
        self.imp.kill()
    }

    /// Return group process identifier.
    ///
    /// On Unix this will be the `pgid` of the process group. On Windows the
    /// default implementation of [`std::process:Child#id`] is used.
    pub fn id(&self) -> u32 {
        self.imp.id()
    }

    /// Waits for the process group to exit completely, returning the status that it
    /// exited with.
    pub fn wait(&mut self) -> Result<ExitStatus> {
        self.imp.wait()
    }

    /// Attempts to collect the exit status of the process group if it has already
    /// exited.
    ///
    /// This function will not block the calling thread and will only check to
    /// see if the child process has exited or not.
    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        self.imp.try_wait()
    }
}

#[cfg(target_family = "unix")]
mod imp {
    use super::{ChildExt, CommandExt, GroupChild};
    use nix::errno::Errno;
    use nix::libc::{self, c_int};
    use nix::sys::signal::{kill, killpg, Signal};
    use nix::sys::wait::{WaitPidFlag, WaitStatus};
    use nix::unistd::*;
    use std::convert::TryInto;
    use std::io::{Error, Result};
    use std::os::unix::process::{CommandExt as UnixCommandExt, ExitStatusExt};
    use std::process::{Child, Command, ExitStatus};

    #[derive(Debug)]
    pub struct GroupChildImp {
        pgid: libc::pid_t,
        inner: Child,
    }

    impl GroupChildImp {
        /// Sends SIGINT to the pgid of this process.
        pub(crate) fn interrupt(&mut self) -> Result<()> {
            signal_process_group(self.pgid, Signal::SIGINT)
        }

        /// Sends SIGTERM to the pgid of this process.
        pub(crate) fn terminate(&mut self) -> Result<()> {
            signal_process_group(self.pgid, Signal::SIGTERM)
        }

        /// Sends SIGKILL to the pgid of this process.
        pub(crate) fn kill(&mut self) -> Result<()> {
            signal_process_group(self.pgid, Signal::SIGKILL)
        }

        /// Return the pid of the child process.
        #[inline]
        pub fn id(&self) -> u32 {
            self.inner.id()
        }

        pub(crate) fn inner(&mut self) -> &mut Child {
            &mut self.inner
        }

        pub(crate) fn wait(&mut self) -> Result<ExitStatus> {
            loop {
                // This should block unless `WNOHANG` is used.
                match waitpid(Pid::from_raw(-self.pgid), None) {
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

        pub(crate) fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
            match waitpid(Pid::from_raw(-self.pgid), Some(WaitPidFlag::WNOHANG))
            {
                Ok((WaitStatus::Exited(_, _), status)) => {
                    Ok(Some(ExitStatus::from_raw(status)))
                }
                Ok((WaitStatus::Signaled(_, _, _), status)) => {
                    Ok(Some(ExitStatus::from_raw(status)))
                }
                Ok(_) => Ok(None),
                Err(e) => Err(e),
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
    ) -> Result<(WaitStatus, i32)> {
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
                .map_err(Error::from)
                .map(|ws| (ws, status)),
        }
    }

    /// Sends a signal to the process using [libc::kill].
    fn signal_process(pid: u32, sig: Signal) -> Result<()> {
        let pid = Pid::from_raw(
            pid.try_into().expect("u32 -> i32 failed in ChildExt::kill"),
        );
        kill(pid, sig).map_err(Error::from)
    }

    /// Sends a signal to the process group using [libc::killpg].
    fn signal_process_group(pgid: i32, sig: Signal) -> Result<()> {
        let pgid = Pid::from_raw(pgid);
        killpg(pgid, sig).map_err(Error::from)
    }

    impl ChildExt for Child {
        /// Sends SIGINT to the pid of this process.
        fn interrupt(&mut self) -> Result<()> {
            signal_process(self.id(), Signal::SIGINT)
        }

        /// Sends SIGTERM to the pid of this process.
        fn terminate(&mut self) -> Result<()> {
            signal_process(self.id(), Signal::SIGTERM)
        }

        /// Sends SIGKILL to the pid of this process.
        fn kill(&mut self) -> Result<()> {
            signal_process(self.id(), Signal::SIGKILL)
        }
    }

    impl CommandExt for Command {
        fn group_spawn(&mut self) -> Result<GroupChild> {
            // Create a new session for the process by calling `setsid`. The
            // process group ID and session ID of the calling process are
            // set to the PID of the calling process (and thus distinct from
            // the daemon's pgid).
            unsafe {
                self.pre_exec(|| setsid().map(|_| ()).map_err(Error::from));
            }

            let child = self.spawn()?;
            let pgid = child
                .id()
                .try_into()
                .expect("u32 -> i32 failed in CommandExt::group_spawn");

            let imp = GroupChildImp { pgid, inner: child };
            Ok(GroupChild { imp })
        }
    }
}

#[cfg(target_family = "windows")]
mod imp {
    use super::{CommandExt, GroupChild};
    use std::io::Result;
    use std::process::{Child, Command, ExitStatus};

    #[derive(Debug)]
    pub struct GroupChildImp {
        inner: Child,
    }

    /// The Windows implementation is extremely poor and will not do the "right"
    /// thing when it comes to killing a service (the service's subprocesses may
    /// continue to live). This requires an imlementation using JobObjects from
    /// the Windows API.
    impl GroupChildImp {
        pub(crate) fn interrupt(&mut self) -> Result<()> {
            self.inner.kill()
        }

        pub(crate) fn terminate(&mut self) -> Result<()> {
            self.inner.kill()
        }

        pub(crate) fn kill(&mut self) -> Result<()> {
            self.inner.kill()
        }

        #[inline]
        pub fn id(&self) -> u32 {
            self.inner.id()
        }

        pub(crate) fn inner(&mut self) -> &mut Child {
            &mut self.inner
        }

        pub(crate) fn wait(&mut self) -> Result<ExitStatus> {
            self.inner.wait()
        }

        pub(crate) fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
            self.inner.try_wait()
        }
    }

    impl ChildExt for Child {
        fn interrupt(&mut self) -> Result<()> {
            self.kill()
        }

        fn terminate(&mut self) -> Result<()> {
            self.kill()
        }

        fn kill(&mut self) -> Result<()> {
            self.kill()
        }
    }

    impl CommandExt for Command {
        fn group_spawn(&mut self) -> Result<GroupChild> {
            let child = self.spawn()?;
            let imp = GroupChildImp { inner: child };
            Ok(GroupChild { imp })
        }
    }
}
