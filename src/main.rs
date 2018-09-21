use std::env;

mod bot;
mod db;
mod web;

fn main() {
    let args: Vec<String> = env::args().collect();

    let host = find_arg(&args, "host", "0.0.0.0");
    let port = find_arg(&args, "port", "8080");
    let admin_id = find_arg(&args, "admin", "280710651").to_string();

    web::start(&host, &port, &mut bot::BotGlobals::new(admin_id));
}

fn find_arg<'a>(args: &'a Vec<String>, key: &str, default_value: &'a str) -> &'a str {
    for arg in args.iter() {
        if arg.starts_with(key) {
            return &arg[key.len()..];
        }
    }

    return default_value;
}
