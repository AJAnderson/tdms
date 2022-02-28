use eframe::egui::ScrollArea;
use eframe::{egui, epi};
use egui::plot::{Line, Value, Values};
use rfd::FileDialog;
use rstdms::{DataTypeVec, TdmsFile};
use log::debug;

pub struct TemplateApp {
    // Example stuff:
    file_handle: Option<TdmsFile>,
    channel_strings: Vec<String>,
    selected_channel: Option<String>,
    cached_data: Option<DataTypeVec>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            file_handle: None,
            channel_strings: Vec::new(),
            selected_channel: None,
            cached_data: None,
        }
    }
}

// Helper functions for loading channels, calls out to rstdms lib functions
impl TemplateApp {
    fn open_dialog(&mut self) {
        if let Some(path) = FileDialog::new().pick_file() {
            let tdms_file = TdmsFile::open(&path).unwrap();
            //println!("{:?}", tdms_file.tdms_map.all_objects);
            self.file_handle = Some(tdms_file)
        }

        self.populate_channels();
    }

    fn populate_channels(&mut self) {
        for channel in self.file_handle.as_ref().expect("No chans").data_objects() {
            self.channel_strings.push(channel.to_string());
        }
    }
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "TDMS Reader"
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
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
                let scroll_area = ScrollArea::new([false, true]);

                let (current_scroll, max_scroll) = scroll_area.show(ui, |ui| {
                    if self.channel_strings.len() > 0 {
                        for (_i, channel) in self.channel_strings.iter().enumerate() {
                            if ui
                                .add(egui::SelectableLabel::new(
                                    false,
                                    channel.clone().replace("\n", " "), // here we strip new lines for display purposes.
                                ))
                                .clicked()
                            {
                                // copy in channel path (Todo: This could just be a reference to the vector index)
                                self.selected_channel = Some(channel.clone());
                                // print the channel properties (for debugging)
                                let result = self.file_handle.as_mut().unwrap().load_data(&channel);
                                match result {
                                    Ok(data) => {
                                        self.cached_data = Some(data.clone());
                                    }
                                    Err(err) => println!("{}", err),
                                }
                            }
                        }
                    };
                    let margin = ui.visuals().clip_rect_margin;

                    let current_scroll = ui.clip_rect().top() - ui.min_rect().top() + margin;
                    let max_scroll =
                        ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin;
                    (current_scroll, max_scroll)
                }).inner;
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            ui.heading("Main plot");

            // If we have a chan_path then load it if we haven't already

            if let Some(data) = self.cached_data.clone() {
                match &data {
                    DataTypeVec::Double(datavector) => {
                        let iter = datavector.iter().step_by(1);
                        let vecy = (0..iter.len()).zip(iter).map(|(i, val)| {
                            let x = i as f64;
                            Value::new(x, val.clone())
                        });

                        let line = Line::new(Values::from_values_iter(vecy.clone()));
                        egui::plot::Plot::new("Channel")
                            .view_aspect(1.0)
                            .show(ui, |plot_ui| plot_ui.line(line));
                    },
                    DataTypeVec::TdmsString(datavector) => {
                        for elem in datavector {
                            println!("{}", elem);
                        }
                    },
                    DataTypeVec::TimeStamp(datavector) => {
                        for elem in datavector {
                            println!("{}", elem);
                        }
                    },
                    _ => unimplemented!(),
                };
            };

            // Display something
            // let sin = (0..1000).map(|i| {
            //     let x = i as f64 * 0.01;
            //     Value::new(x, x.sin())
            // });
            // let line = Line::new(Values::from_values_iter(sin));
            // ui.add(egui::plot::Plot::new("Channel").line(line).view_aspect(1.0));
            // egui::warn_if_debug_build(ui);
        });
    }
}
