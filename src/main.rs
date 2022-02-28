// #![warn(clippy::all)]
use flexi_logger::{opt_format, Logger};

use rstdms::TdmsError;
use std::env;

mod app;
pub use app::TemplateApp;

fn main() -> Result<(), TdmsError> {
    // Initialize a logger for logging debug messages, useful during prototyping
    // "rstdms=debug, lib=debug"
    Logger::with_env_or_str("rstdms=debug, lib=debug")
        .log_to_file()
        .directory("log_files")
        .format(opt_format)
        .start()
        .unwrap();

    // call with cargo run Example.tdms to run the example
    let args: Vec<String> = env::args().collect();

    println!("{:?}", args);

    // Create the gui stuff
    let app = TemplateApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);

    Ok(())
}
