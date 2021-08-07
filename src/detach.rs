use nix::libc::{
    c_int, exit, fork, getpgrp, getpid, setsid, wait, WEXITSTATUS, WIFEXITED,
};
use nix::unistd::execvp;
use std::env::Args;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::panic;

pub fn detach_tty(args: Args, wait_child: bool) {
    // Don't undwind or run the panic hook. Since we fork() it isn't safe on
    // many platforms to call the allocator.
    panic::always_abort();

    let argv = args
        .skip(1)
        .filter(|arg| arg != "--detach" && arg != "-d")
        .map(|s| CString::new(s).unwrap())
        .collect::<Vec<_>>();

    let current_exe_path = CString::new(
        std::env::current_exe()
            .expect("Failed to get current exe path")
            .into_os_string()
            .as_bytes(),
    )
    .expect("Failed to get Cstr");

    unsafe {
        if getpgrp() == getpid() {
            let mut wstatus: i32 = 0;

            let pid = fork();

            match fork() {
                -1 => panic!("failed during fork"),
                0 => { /* child */ }
                _ => {
                    /* parent */
                    if !wait_child {
                        exit(0);
                    }
                    if wait(&mut wstatus as *mut c_int) != pid {
                        panic!("failed to wait child")
                    }
                    if WIFEXITED(wstatus) {
                        exit(WEXITSTATUS(wstatus));
                    }
                    panic!("child did not exit normally")
                }
            }
        }

        // `setsid` will create a new session for the process and the new
        // session will have no controlling terminal, effectively
        // dissassociating it from its parents controlling terminal
        if setsid() < 0 {
            panic!("failed to setsid");
        }
    }

    execvp(&current_exe_path, &argv).unwrap();
}
