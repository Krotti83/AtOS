use crate::stdlib::syscalls::{fork, exec, wait, sys_write};
use core::{fmt, fmt::Write};

/* ~~~ STDIO ~~~ */
// For printing or getting input from the stdio (UART).
// printing is assigned syscall number 1 (svc #1),
// and getting input is assigned syscall number 2 (svc #2)
// \TODO INPUT HANDLING
pub struct Stdout;

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        sys_write(s);
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) -> fmt::Result {
    Stdout.write_fmt(args)
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::stdlib::syswraps::_print(
            core::format_args!($($arg)*)
        ).unwrap()
    });
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        $crate::stdlib::syswraps::_print(
            core::format_args!(
                "{}\n",
                core::format_args!($($arg)*)
            )
        ).unwrap()
    });
}

pub fn spawn(path: &str, args: &[&str]) -> Result<(), &'static str> {
    match fork() {
        Ok(fc) => {
            if fc == 0 {
                exec(path, args)?;
            } else {
                wait(Some(fc))?;
            }
        }
        Err(_) => {
            return Err("fork failed");
        }
    }

    Ok(())
}