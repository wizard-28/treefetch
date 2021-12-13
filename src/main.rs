use std::process::Command;
use std::io::Read;
use std::env;
use std::fs;
use regex::{Regex, Captures};
mod colors;

// Simple system fetch tool written in Rust.
fn main() {
    let ascii_tree = format!("{green}     /\\*\\       {reset}
{green}    /\\O\\*\\      {reset}
{green}   /*/\\/\\/\\     {reset}
{green}  /\\O\\/\\*\\/\\    {reset}
{green} /\\*\\/\\*\\/\\/\\   {reset}
{green} |O\\/\\/*/\\/O|   {reset}
{yellow}      ||        {reset}
{yellow}      ||        {reset}
",
    green = colors::green,
    yellow = colors::yellow,
    reset = colors::reset,
);
    let ascii_tree = split_by_newline(ascii_tree);

    let mut data_list: Vec<String> = Vec::new();

    // Username / Hostname

    {
        let username_env = env::var_os("USER");
        let username: String;

        if username_env.is_some() {
            username = username_env.unwrap().into_string().unwrap();
        } else {
            username = String::new();
        }

        let mut hostname_file = fs::File::open("/etc/hostname").unwrap();
        let mut hostname = String::new();

        let result = hostname_file.read_to_string(&mut hostname);

        if result.is_ok() {
            let user_host_name = format!("{color}{bold}{user}{reset}
                                         {bold}@{color}{host}{reset}",
                                         user = username,
                                         host = hostname,
                                         color = colors::green,
                                         bold = colors::bold,
                                         reset = colors::reset,
                                         ).replace(" ", "").replace("\n", "");
            data_list.push(user_host_name);

            // Separator
            // format: username length + @ (1) + hostname length

            let user_host_name_len = username.len() + 1 + hostname.len();
            let mut separator = String::new();

            separator += colors::yellow;
            for _i in 0..(user_host_name_len) {
                separator += "━";
            }
            separator += colors::reset;

            data_list.push(separator);
        }
    }

    // Distro name

    let distro_data = run_command("/bin/sh", vec!("-c",
                                                  "cat /etc/*-release",
                                                  ));
    let re_distro = match_regex(&distro_data,
                                r#"(?x)
                                DISTRIB_DESCRIPTION=
                                "?   # Quotes if description is multiple words
                                (?P<distro_name>[^\n"]+)
                                "?   # Ditto
                                \n
                                "#.to_string());

    if !re_distro.is_none() {
        let re_distro = re_distro.unwrap();

        let distro_name = re_distro.name("distro_name").unwrap().as_str();
        data_list.push(format_data("os", &distro_name));
    }

    // Kernel name

    let kernel = run_command("uname", vec!("-mrs"));
    let re_kernel = match_regex(&kernel,
                                r#"(?x)
                                (?P<kernel_name>\S+)
                                \s+
                                (?P<kernel_version>\S+)"#.to_string());

    if !re_kernel.is_none() {
        let re_kernel = re_kernel.unwrap();

        let kernel = re_kernel.name("kernel_version").unwrap().as_str();
        data_list.push(format_data("kernel", &kernel));
    }

    // Shell

    let shell = run_command("/bin/sh", vec!("-c",
                                            "echo $SHELL"));
    let re_shell = match_regex(&shell,
                               r#"(?x)
                               (?P<shell_name>[^/]+)$
                               "#.to_string());

    if !re_shell.is_none() {
        let re_shell = re_shell.unwrap();

        let shell = re_shell.name("shell_name").unwrap().as_str();
        data_list.push(format_data("shell", &shell));
    }

    // Uptime

    let uptime = run_command("cat", vec!("/proc/uptime"));
    let re_uptime = match_regex(&uptime,
                                r#"(?x)
                                ^(?P<uptime_seconds>\d+)\.
                                "#.to_string());

    if !re_uptime.is_none() {
        let re_uptime = re_uptime.unwrap();

        let uptime_seconds: u32 = re_uptime
            .name("uptime_seconds")
            .unwrap()
            .as_str()
            .parse()
            .unwrap();
        let uptime_hours: u32 = uptime_seconds / (60 * 60);
        let uptime_minutes: u32 = (uptime_seconds % (60 * 60)) / 60;
        data_list.push(format_data(
                "uptime",
                &format!("{hours}h {minutes}m",
                         hours = uptime_hours,
                         minutes = uptime_minutes)
                ));
    }

    // Memory

    let memory = run_command("free", vec!("-m"));
    let re_memory = match_regex(&memory,
                                r#"(?x)
                                Mem:
                                \s+
                                (?P<total>\d+)
                                \s+
                                (?P<used>\d+)
                                "#.to_string());
    if !re_memory.is_none() {
        let re_memory = re_memory.unwrap();

        let total_mem = re_memory.name("total").unwrap().as_str();
        let used_mem = re_memory.name("used").unwrap().as_str();
        data_list.push(format_data(
                "memory",
                &format!("{used}m / {total}m",
                         used = used_mem,
                         total = total_mem)
                ));
    }

    print_left_to_right(ascii_tree, data_list);
}

// Print two vectors of strings side to side
fn print_left_to_right(left: Vec<String>, right: Vec<String>) {
    let left_len = left.len();
    let right_len = right.len();
    let max_len = if left_len > right_len {left_len} else {right_len};

    for i in 0..max_len {
        if i < left_len {
            print!("{}", left[i]);
        }
        if i < right_len {
            print!("{}", right[i]);
        }

        // Print a newline
        println!("");
    }
}

// Split a multi-line string into several ones separated by the newline
fn split_by_newline(ascii_art: String) -> Vec<String> {
    let mut split: Vec<String> = Vec::new();
    let mut last_index = 0;

    let bytes = ascii_art.as_bytes();

    for (i, &item) in bytes.iter().enumerate() {
        if item == b'\n' {
            split.push(ascii_art[last_index..i].trim().to_string());
            last_index = i;
        }
    }

    split
}

fn format_data(key: &str, value: &str) -> String {
    format!("{color}▪{bold} {key:7}{reset} {value}",
            key = key,
            value = value,
            color = colors::green,
            bold = colors::bold,
            reset = colors::reset,
            )
}

// Search with Regex in a string and return all of the matches
fn match_regex(search_str: &String, regex: String) -> Option<Captures> {
    let re = Regex::new(&regex).unwrap();

    re.captures(&search_str)
}

// Run a command and return the output
fn run_command(command: &str, args: Vec<&str>) -> String {
    // Initialize the process
    let mut command = Command::new(command);
    // Add the arguments
    command.args(args);

    // Run the command
    let output = command
                 .output()
                 .expect("failed to execute process");

    // Return the output (stdout)
    let stdout = String::from_utf8(output.stdout)
                 .unwrap();
    stdout.trim().to_string()
}
