use eframe::egui::ScrollArea;
use eframe::{egui, epi};
// use eframe::egui::Ui;
use egui::plot::{Legend, Line, Plot, Text, Value, Values};
use egui::Align2;
use log::debug;
use rfd::FileDialog;
use std::collections::HashMap;
use std::error::Error;
use tdms::{DataTypeVec, TdmsFile};

pub struct ChannelState {
    name: String,
    selected: bool,
}

pub struct ScryApp {
    // Example stuff:
    file_handle: Option<TdmsFile>,
    channel_state: Vec<ChannelState>,
    cached_data: HashMap<String, DataTypeVec>,
}

impl Default for ScryApp {
    fn default() -> Self {
        Self {
            file_handle: None,
            channel_state: Vec::new(),
            cached_data: HashMap::new(),
        }
    }
}

// Helper functions for loading channels, calls out to rstdms lib functions
impl ScryApp {
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
            self.channel_state.push(ChannelState {
                name: channel.to_string(),
                selected: false,
            });
        }
    }

    fn cached_data_to_line(&mut self) -> Option<Vec<Line>> {
        let mut out_lines: Vec<Line> = Vec::new();

        for (name, data) in self.cached_data.iter() {
            let double_data = Vec::<f64>::try_from(data.clone()).expect("Unimplemented datatype");
            let iter = double_data.iter().step_by(1);
            let vecy = (0..iter.len()).zip(iter).map(|(i, val)| {
                let x = i as f64;
                Value::new(x, *val)
            });
            out_lines.push(Line::new(Values::from_values_iter(vecy.clone())).name(name))
        }

        Some(out_lines)
    }
}

impl epi::App for ScryApp {
    fn name(&self) -> &str {
        "Scry TDMS Reader"
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
                ui.heading("Channels");

                if ui.button("Load File").clicked() {
                    self.open_dialog()
                }
                let scroll_area = ScrollArea::new([false, true]).id_source("area1");

                let (_current_scroll, _max_scroll) = scroll_area
                    .show(ui, |ui| {
                        if self.channel_state.len() > 0 {
                            for channel in self.channel_state.iter_mut() {
                                ui.horizontal(|ui| {
                                    ui.label(channel.name.clone().replace("\n", " "));
                                    if ui.checkbox(&mut channel.selected, "").changed() {
                                        if channel.selected {
                                            let result = self
                                                .file_handle
                                                .as_mut()
                                                .unwrap()
                                                .load_data(&channel.name);
                                            match result {
                                                Ok(data) => {
                                                    self.cached_data
                                                        .insert(channel.name.clone(), data.clone());
                                                }
                                                Err(err) => println!("{}", err),
                                            }
                                        } else {
                                            self.cached_data.remove_entry(&channel.name);
                                        }
                                    }
                                });
                            }
                        };
                        let margin = ui.visuals().clip_rect_margin;

                        let current_scroll = ui.clip_rect().top() - ui.min_rect().top() + margin;
                        let max_scroll =
                            ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin;
                        (current_scroll, max_scroll)
                    })
                    .inner;
            });

        egui::SidePanel::right("side_panel")
            .min_width(200.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Channel Properties");
                for channel in self.channel_state.iter() {
                    if channel.selected {
                        if let Some(file_map) = &self.file_handle {
                            let properties = file_map
                                .object_properties(&channel.name)
                                .expect("TODO (Error handling): No props");
                            ui.label(properties.to_string());
                        }
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Main Plot Pannel
            ui.heading("Numeric Data");

            // If we have a chan_path then load it if we haven't already
            if let Some(lines) = self.cached_data_to_line() {
                Plot::new("Channel Data")
                    .legend(Legend::default())
                    // .x_axis_formatter(|value, range| {
                    //         format!("hello: {}", value).to_string()
                    //      })
                    .show(ui, |plot_ui| {
                        for line in lines {
                            plot_ui.line(line)
                        }
                        plot_ui.text(
                            Text::new(Value::new(0.0, 0.0), "Time").anchor(Align2::CENTER_TOP),
                        )
                    });
            }

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
