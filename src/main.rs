use flexi_logger::{opt_format, Logger};
mod tdms_error;
use std::env;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path;
// use tdms_error::{TdmsError, TdmsErrorKind};

use rstdms::{TdmsFile, TdmsError, TdmsErrorKind};

fn main() -> Result<(), TdmsError> {
    // Initialize a logger for logging debug messages, useful during prototyping
    Logger::with_env_or_str("rstdms=debug, lib=debug")
        .log_to_file()
        .directory("log_files")
        .format(opt_format)
        .start()
        .unwrap();

    // call with cargo run Example.tdms to run the example
    let args: Vec<String> = env::args().collect();

    println!("{:?}", args);

    let fname = &args[1];

    // Convert string to path and load file handle into struct
    let pathbuf = path::PathBuf::from(fname);

    println!();
    println!("-----------------------------------------");
    println!("Loading TDMS File {}", fname);
    println!("-----------------------------------------");
    println!();
    let mut tdms_file = TdmsFile::new_file(&pathbuf)?;

    match tdms_file.map_segments() {
        Ok(_) => (),
        Err(e) => {
            tdms_file.current_loc();
            return Err(e);
        }
    }

    let channels = tdms_file.objects();
    for channel in channels {
        println!("{:?}", channel);
    }

    let data = tdms_file.load_data("/'Untitled'/'Time Stamp'")?;
    // let data = tdms_file.load_data("Baratron ChamberPressure >1500Pa")?;

    println!("{:?}", data);

    Ok(())
}
