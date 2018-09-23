#[macro_use(params)]
extern crate mysql;

use std::env;

mod bot;
mod db;
mod web;

fn main() {
    let args: Vec<String> = env::args().collect();

    let host = find_arg(&args, "host", "0.0.0.0");
    let port = find_arg(&args, "port", "8080");
    let admin_id = find_arg(&args, "admin", "280710651");
    let db_user = find_arg(&args, "db_user", "root");
    let db_pwd = find_arg(&args, "db_pwd", "");

    db::init(&db_user, &db_pwd);
    web::start(host, port, &mut bot::BotGlobals::new(admin_id, db_user, db_pwd));
}

fn find_arg<'a>(args: &'a Vec<String>, key: &str, default_value: &'a str) -> String {
    for arg in args.iter() {
        if arg.starts_with(key) {
            return String::from(&arg[key.len() + 1..]);
        }
    }

    return String::from(default_value);
}
