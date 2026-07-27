#![no_std]
#![no_main]

use user::entry_args;
use user::println;

pub fn main(args: &[&str]) {

    if args.len() < 2 {
        println!("Usage: help <command or program name>");

        println!("Available commands/programs:");
        println!("  init - The initial program that starts the shell (executed by kernel)");
        println!("  help - Display this help message");
        println!("  info - Display information about the operating system");
        println!("  echo - Display a line of text");
        println!("  clear - Clear the terminal screen");
        println!("  ptest - A test program for process management");
        println!("  waittest - A test program for waiting on child processes");
        println!("  exectest - A test program for executing other programs");
        println!("  wc - Count lines, words, and bytes in input");
        println!("  tetris - Play a game of Tetris on AtOS");
        println!("  exit - Exit the shell (implemented in shell itself)");
        println!("Filesystem related commands will be implemented later with filesystem itself.");
        println!("\nAvailable syntax for commands/programs:");
        println!(" command [arguments] - Execute a command with optional arguments");
        println!(" command [arguments] & - Execute a command with optional arguments in");
        println!("                         the background");
        println!(" command1 [arguments] & command2 [arguments] - Execute command1 in the ");
        println!("                                               background and command2 in");
        println!("                                               the foreground");
        println!(" command1 [arguments]; command2 [arguments] - Execute command1 and then");
        println!("                                              command2 in sequence");
        println!(" command1 \"argument\" - Execute command1 with an argument that contains");
        println!("                         spaces (enclosed in quotes)");

        return;
    } else {
        let command = args[1];

        match command {
            "init" => println!("\
                [init] \
            \n| The initial program that is spawned by the kernel. It spawns the shell, \
            \n| and any other programs needed. Orphan processes are reparented to this \
            \n| program and it reaps them continuously. \
            "),
            "help" => println!("\
                [help] \
            \n| You are using this right now. It shows help information for commands and programs \
            "),
            "info" => println!("\
                [info] \
            \n| Display information about the operating system. Inspired by Linux's `fastfetch`. \
            "),
            "echo" => println!("\
                [echo] \
            \n| Display a line of text. Simply writes back arguments given to it. \
            "),
            "clear" => println!("\
                [clear] \
            \n| Clear the terminal screen. \
            "),
            "ptest" => println!("\
                [ptest] \
            \n| A test program for process management. Tests forking, execution, waiting, \
            \n| and termination. It spawns `waittest` and `exectest` programs as well. \
            "),
            "waittest" => println!("\
                [waittest] \
            \n| A test program for waiting on child processes. Tests the wait() syscall \
            "),
            "exectest" => println!("\
                [exectest] \
            \n| A test program for executing other programs. Tests the exec() syscall \
            "),
            "wc" => println!("\
                [wc] \
            \n| Count lines, words, and bytes in input. \
            "),
            "tetris" => println!("\
                [tetris] \
            \n| Play a game of Tetris on AtOS! \
            "),
            "exit" => println!("\
                [exit] \
            \n| Exit the shell. This command is implemented in the shell itself, not as \
            \n| a separate program. \
            "),
            _ => println!("Unknown command/program: {}", command),
        }
    }
}

entry_args!(main);
