//Given an excel file, read it and store the dates and times that jen works.
use log::{debug, error, info, warn};
use schedule_reader::Config;
use std::{env, process};

//use serde::{Deserialize};

fn main() {
    env_logger::init();
    error!("ERROR");
    warn!("WARN");
    info!("INFO");
    debug!("DEBUG\n\n");

    let config = Config::new(env::args()).unwrap_or_else(|e| {
        println!("Problem parsing arguments: {}", e);
        process::exit(1);
    });

    if let Err(e) = schedule_reader::process_schedule(config) {
        error!("Application error: {}", e);
        process::exit(1);
    }
}
