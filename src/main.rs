use eframe::egui::{vec2, Vec2};
// #![warn(clippy::all)]
use flexi_logger::{opt_format, Logger};
mod tdms_error;
use charts::{Chart, LineSeriesView, MarkerType, PointDatum, ScaleLinear};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path;
// use tdms_error::{TdmsError, TdmsErrorKind};
use rstdms::{DataTypeVec, TdmsError, TdmsFile};

mod app;
pub use app::TemplateApp;

fn main() -> Result<(), TdmsError> {
    // Initialize a logger for logging debug messages, useful during prototyping
    // "rstdms=debug, lib=debug"
    Logger::with_env_or_str("rstdms=error, lib=error")
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
    let mut native_options = eframe::NativeOptions::default();
    // native_options.initial_window_size = Some(eframe::egui::vec2(1920.0, 1024.0));
    eframe::run_native(Box::new(app), native_options);

    // This is a dummy to let me turn off plotting without having to comment everything out
    // let data = DataTypeVec::Void(Vec::new());

    // //Prepare a chart using the charts library
    // // Define chart related sizes.
    // let width = 800;
    // let height = 600;
    // let (top, right, bottom, left) = (90, 40, 50, 60);

    // // For dev purposes, setting up a special case struct to be able to do conversions
    // struct Point(usize, f64);

    // impl PointDatum<f32, f32> for Point {
    //     fn get_x(&self) -> f32 {
    //         self.0 as f32
    //     }

    //     fn get_y(&self) -> f32 {
    //         self.1 as f32
    //     }

    //     fn get_key(&self) -> String {
    //         String::new()
    //     }
    // }

    // match data {
    //     DataTypeVec::Double(inner) => {
    //         println!("Data length {}", inner.len());
    //         println!("Last value {}", inner.last().unwrap());

    //         // Chart axis preparation code - Create a band scale that will interpolate values in [0, data length] to values in the [0, availableWidth] range (the width of the chart without the margins).
    //         let x = ScaleLinear::new()
    //             .set_domain(vec![0.0, inner.len() as f32])
    //             .set_range(vec![0, width - left - right]);

    //         // Create a linear scale that will interpolate values in [0, 1] range to corresponding values in [availableHeight, 0] range (the height of the chart without the margins). The [availableHeight, 0] range is inverted because SVGs coordinate system's origin is in top left corner, while chart's origin is in bottom left corner, hence we need to invert the range on Y axis for the chart to display as though its origin is at bottom left.
    //         let y = ScaleLinear::new()
    //             .set_domain(vec![0.0, 1.0])
    //             .set_range(vec![height - top - bottom, 0]);

    //         // TODO The data needs to implement the "point datum" trait, try to zip an integer series with our test float data
    //         let indices: Vec<usize> = (1..inner.len()).collect();
    //         let line_data = indices
    //             .into_iter()
    //             .zip(inner)
    //             .map(|(x, y)| Point(x, y))
    //             .collect();

    //         // Create Line series view that is going to represent the data.
    //         let line_view = LineSeriesView::new()
    //             .set_x_scale(&x)
    //             .set_y_scale(&y)
    //             .set_marker_type(MarkerType::Circle)
    //             .set_label_visibility(false)
    //             .load_data(&line_data)
    //             .unwrap();

    //         // Generate and save the chart.
    //         Chart::new()
    //             .set_width(width)
    //             .set_height(height)
    //             .set_margins(top, right, bottom, left)
    //             .add_title(String::from("Line Chart"))
    //             .add_view(&line_view)
    //             .add_axis_bottom(&x)
    //             .add_axis_left(&y)
    //             .add_left_axis_label("Custom Y Axis Label")
    //             .add_bottom_axis_label("Custom X Axis Label")
    //             .save("line-chart-sigexp.svg")
    //             .unwrap();

    //         // print!("{:?}", inner);
    //     }
    //     _ => println!("Not implemented"),
    // }

    // println!("{}", data.len());

    Ok(())
}
