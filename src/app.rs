use eframe::{egui, epi};
use egui::plot::{Line, Plot, Value, Values};
use rfd::FileDialog;
use rstdms::{DataTypeVec, TdmsError, TdmsFile};
use std::path::PathBuf;

pub struct TemplateApp {
    // Example stuff:
    label: String,
    filepath: PathBuf,
    file_handle: Option<TdmsFile>,
    value: f32,
    channel_strings: Vec<String>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            filepath: PathBuf::default(),
            file_handle: None,
            value: 2.7,
            channel_strings: Vec::new(),
        }
    }
}
impl TemplateApp {
    fn open_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            let tdms_file = TdmsFile::open(&path).unwrap();
            self.file_handle = Some(tdms_file)
        }

        self.populate_channels();
    }

    fn populate_channels(&mut self) {
        for channel in self.file_handle.as_ref().expect("No chans").objects() {
            println!("{:?}", channel.clone());
            self.channel_strings.push(channel.to_string());
        }
    }
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "egui template"
    }

    /// Called by the framework to load old app state (if any).
    // #[cfg(feature = "persistence")]
    // fn setup(
    //     &mut self,
    //     _ctx: &egui::CtxRef,
    //     _frame: &mut epi::Frame<'_>,
    //     storage: Option<&dyn epi::Storage>,
    // ) {
    //     if let Some(storage) = storage {
    //         *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
    //     }
    // }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel")
            .min_width(200.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Side Panel");

                if ui.button("Load File").clicked() {
                    self.open_dialog()
                }
                if self.channel_strings.len() > 0 {
                    for (i, channel) in self.channel_strings.iter().enumerate() {
                        ui.add(egui::SelectableLabel::new(
                            false,
                            channel.clone().replace("\n", " "),
                        ));
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            ui.heading("Main plot");

            // Display something
            // let sin = (0..1000).map(|i| {
            //     let x = i as f64 * 0.01;
            //     Value::new(x, x.sin())
            // });
            // let line = Line::new(Values::from_values_iter(sin));
            // ui.add(egui::plot::Plot::new("Channel").line(line).view_aspect(1.0));
            // egui::warn_if_debug_build(ui);
        });

        if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally chose either panels OR windows.");
            });
        }
    }
}
