#![no_std]
#![no_main]

use user::entry_args;
use user::stdlib; // you may find common user methods in this module.

pub fn main(args: &[&str]) {
    // This is a template for creating new programs which need to use 
    // entry arguments in the AtOS user space.
    // You can use this as a starting point for your own programs.
    // Remember to index your program in `kernel::filesystem::read_file`
    // make already includes all files in `src/bin`! so you need not do anything else. 

    // Your code goes here...
}

entry_args!(main);
