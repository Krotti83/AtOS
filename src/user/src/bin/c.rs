#![no_std]
#![no_main]

use user::{entry, println, stdlib::syscalls::{fork, wait}};

fn main() {
    println!("hello this code is running in the c program!");

    let mut x = 1;
    println!("x = {}", x);
    x += 1;
    println!("x = {}", x);

    for i in 0..20 {
        println!("c program is working, iteration {}", i);
    }

    println!("c program will now fork and wait for the child to finish.");
    match fork() {
        Ok(fc) => {
            if fc == 0 {
                println!("i'm c child! i finished early!");
                panic!("Hi im c child. I just paniced here to check if my parent receives my exit code 1.");
            } else {
                println!("I'm c parent, now working");
                for i in 0..15 {
                    println!("c parent working {}", i);
                }
                match wait(None) {
                    Ok((pid, exit_code)) => {
                        println!("c parent waited for child process with pid {} to finish, it exited with code {}", pid, exit_code);
                    }
                    Err(e) => {
                        println!("c parent program failed to wait for child process: {}", e);
                    }
                }
                println!("c parent will now exit.");
            }
        }
        Err(_) => {
            println!("fork() failed!");
        }
    }
}


entry!(main);