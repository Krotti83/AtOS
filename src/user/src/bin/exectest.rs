// process B just for the sake of testing the scheduler with init and process B

#![no_std]
#![no_main]

use user::{entry, println};
use user::stdlib::syscalls::{fork, exec};

fn parent_do() {
    println!("This is the parent process doing work!");

    for i in 0..20 {
        println!("parent process is working, iteration {}", i);
    }
    
    println!("parent process is done working, it will now exit.");
}

fn child_do() {
    println!("This is the child process doing work!");

    for i in 0..10 {
        println!("child process is working, iteration {}", i);
    }
    println!("child process is done working, it will now attempt exec(\"init\")");
    exec("init", &[]).unwrap();
}

fn main() {
    println!("hello this code is running in process B!");
    
    println!("Trying to fork in B!");

    match fork() {
        Ok(fc) => {
            if fc == 0 {
                child_do();
            } else {
                parent_do();
            }
        }
        Err(_) => {
            println!("fork() failed!");
        }
    }
}

entry!(main);