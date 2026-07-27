#![no_std]
#![no_main]

use user::{entry_args, println, stdlib::syscalls::{fork, wait}};
use user::stdlib::syswraps::spawn;

fn main(args: &[&str]) {
    println!("hello this code is running in the 'a' program!");
    println!("args passed to 'a' program: {:?}", args);

    let mut x = 1;
    println!("x = {}", x);
    x += 1;
    println!("x = {}", x);

    for i in 0..20 {
        println!("'a' program is working, iteration {}", i);
    }

    println!("'a' program will now fork and wait for the child to finish.");
    match fork() {
        Ok(fc) => {
            if fc == 0 {
                for i in 0..10 {
                    println!("child working {}", i);
                }
                println!("child is done working, it will now exit.");
            } else {
                println!("I'm parent, now waiting for child to finish");
                match wait(None) {
                    Ok((pid, exit_code)) => {
                        println!("parent waited for child process with pid {} to finish, it exited with code {}", pid, exit_code);
                    }
                    Err(e) => {
                        println!("parent program failed to wait for child process: {}", e);
                    }
                }
                println!("parent will now exit.");
                println!("before exiting, spawning process c for next test.");
                spawn("c", &[]).unwrap();
            }
        }
        Err(_) => {
            println!("fork() failed!");
        }
    }
}


entry_args!(main);