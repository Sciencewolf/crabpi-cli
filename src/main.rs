extern crate core;

use std::env;

fn get_all_files() -> Vec<String> {
    return vec![];
}

fn get_file(filename: &str) -> String {
    return String::new();
}

fn upload_file(filepath: &str) -> String {
    return String::new();
}

fn delete_file(filename: &str) -> String {
    return String::new();
}

fn rename_file(old: &str, new: &str) -> String {
    return String::new()
}

fn preview_file(filename: &str) -> String {
    return String::new();
}

enum KEYWORDS {
    LS,
    PREVIEW,
    DEL,
    UP,
    GET
}


fn main() {
    let args: Vec<String> = env::args().collect();

    println!("args: \n");
    for arg in &args {
        println!("{:?}", arg);
    }


    // match KEYWORDS::DEL {
    //     KEYWORDS::LS => println!("{:?}", get_all_files()),
    //     _ => println!("Invalid command")
    // }
}