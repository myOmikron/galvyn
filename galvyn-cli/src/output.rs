use owo_colors::colored::Color;
use owo_colors::OwoColorize;

pub fn print_info(txt: &str) {
    println!("{} {txt}", "[INFO]".color(Color::Cyan));
}

pub fn print_err(txt: &str) {
    println!("{} {txt}", "[ERROR]".color(Color::Red));
}
