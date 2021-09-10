use eframe::egui::ScrollArea;
use eframe::{egui, epi};
use egui::plot::{Line, Value, Values};
use rfd::FileDialog;
use rstdms::{DataTypeVec, TdmsError, TdmsFile};
use std::path::PathBuf;

pub struct TemplateApp {
    // Example stuff:
    file_handle: Option<TdmsFile>,
    channel_strings: Vec<String>,
    selected_channel: Option<String>,
    cached_data: Option<Values>,
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
                let scroll_area = ScrollArea::auto_sized();

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
                            }
                        }
                    };
                    let margin = ui.visuals().clip_rect_margin;

                    let current_scroll = ui.clip_rect().top() - ui.min_rect().top() + margin;
                    let max_scroll =
                        ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin;
                    (current_scroll, max_scroll)
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            ui.heading("Main plot");

            // If we have a chan_path then load it if we haven't already
            if let Some(chan_path) = self.selected_channel.clone() {
                match self.file_handle.as_mut().unwrap().load_data(&chan_path) {
                    Ok(data) => match &data {
                        DataTypeVec::Double(datavector) => {
                            let vecy = (0..datavector.len()).map(|i| {
                                let x = i as f64;
                                Value::new(x, datavector[i])
                            });

                            let line = Line::new(Values::from_values_iter(vecy));
                            ui.add(egui::plot::Plot::new("Channel").line(line).view_aspect(1.0));
                        }
                        _ => unimplemented!(),
                    },
                    _ => unimplemented!(),
                }
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
