#![no_std]
#![no_main]

use user::{entry, println, stdlib::syscalls::{fork, wait}};

fn main() {
    println!("hello this code is running in the waittest program!");

    let mut x = 1;
    println!("x = {}", x);
    x += 1;
    println!("x = {}", x);

    for i in 0..20 {
        println!("waittest program is working, iteration {}", i);
    }

    println!("waittest program will now fork and wait for the child to finish.");
    match fork() {
        Ok(fc) => {
            if fc == 0 {
                println!("i'm waittest child! i finished early!");
                panic!("Hi im waittest child. I just paniced here to check if my parent receives my exit code 1.");
            } else {
                println!("I'm waittest parent, now working");
                for i in 0..15 {
                    println!("waittest parent working {}", i);
                }
                match wait(None) {
                    Ok((pid, exit_code)) => {
                        println!("waittest parent waited for child process with pid {} to finish, it exited with code {}", pid, exit_code);
                    }
                    Err(e) => {
                        println!("waittest parent program failed to wait for child process: {}", e);
                    }
                }
                println!("waittest parent will now exit.");
            }
        }
        Err(_) => {
            println!("fork() failed!");
        }
    }
}


entry!(main);