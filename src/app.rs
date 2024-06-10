use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;

use eframe::epaint::Color32;
use egui::{Align2, Pos2, RichText, Vec2};
use log::{debug, error, info};
use strum::{EnumString, IntoEnumIterator, IntoStaticStr};
use walkdir::WalkDir;

use crate::image_converter::convert;

#[derive(Debug, PartialEq, Clone, EnumString, IntoStaticStr, Copy)]
pub enum ImageFormatEnum {
    #[strum(serialize = "png")]
    PNG,
    #[strum(serialize = "dds")]
    DDS,
}

pub(crate) struct ImageConverterApp {
    selected_source_dir: Option<String>,
    selected_dest_dir: Option<String>,
    files: Option<Vec<String>>,
    output_format: ImageFormatEnum,
    dds_format: image_dds::ImageFormat,
    selected_row_index: i8,
    is_window_open: bool,
    is_convert_success: Option<bool>,
    is_debug_panel_open: bool,
    set_window_open_flag: bool,
    tx: Sender<bool>,
    rx: Receiver<bool>,
}

impl eframe::App for ImageConverterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        // main panel
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!self.is_window_open);

            ui.horizontal(|ui| {
                if ui.button("Select folder").clicked() {
                    let files = rfd::FileDialog::new().set_directory("/").pick_folder();

                    if files.is_some() {
                        self.selected_source_dir =
                            Some(files.unwrap().to_str().unwrap().to_string());
                        self.files = None;
                        info!("source dir: {:?}", self.selected_source_dir);
                    }
                }

                ui.label(format!("Source folder: {:?}", self.selected_source_dir));
            });
            ui.horizontal(|ui| {
                if ui.button("Select folder").clicked() {
                    let files = rfd::FileDialog::new().set_directory("/").pick_folder();

                    if files.is_some() {
                        self.selected_dest_dir = Some(files.unwrap().to_str().unwrap().to_string());
                        info!("dest dir: {:?}", self.selected_source_dir);
                    }
                }

                ui.label(format!("Destination folder: {:?}", self.selected_dest_dir));
            });

            if self.selected_source_dir.is_some() && self.files.is_none() {
                let selected_source_dir = self.selected_source_dir.clone();

                self.files = Some(
                    WalkDir::new(selected_source_dir.unwrap())
                        .into_iter()
                        .filter_map(|file| file.ok())
                        .filter(|file| file.metadata().unwrap().is_file())
                        .filter(|file| {
                            let array: [String; 3] = [
                                "dds".parse().unwrap(),
                                "png".parse().unwrap(),
                                "jpg".parse().unwrap(),
                            ];
                            array.contains(
                                &file
                                    .path()
                                    .extension()
                                    .unwrap_or("None".as_ref())
                                    .to_str()
                                    .unwrap()
                                    .to_string(),
                            )
                        })
                        .map(|file| String::from(file.path().to_str().unwrap()))
                        .collect(),
                );
                debug!("files: {:?}", self.files);
            }

            ui.horizontal(|ui| {
                egui::ComboBox::from_label("Output Format")
                    .selected_text(format!("{:?}", self.output_format))
                    .show_ui(ui, |ui| {
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        ui.selectable_value(&mut self.output_format, ImageFormatEnum::PNG, "PNG");
                        ui.selectable_value(&mut self.output_format, ImageFormatEnum::DDS, "DDS");
                    });

                if self.output_format == ImageFormatEnum::DDS {
                    egui::ComboBox::from_label("DDS Format")
                        .selected_text(format!("{:?}", self.dds_format))
                        .show_ui(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.set_min_width(200.0);

                            for format in image_dds::ImageFormat::iter() {
                                ui.selectable_value(
                                    &mut self.dds_format,
                                    format,
                                    format.to_string(),
                                );
                            }
                        });
                }
            });

            ui.add_enabled_ui(
                self.files.as_ref().map_or(false, |vec| !vec.is_empty()),
                |ui| {
                    if ui.button("Convert").clicked() {
                        self.is_convert_success = None;
                        self.is_window_open = true;

                        let files = self.files.clone().unwrap();
                        let selected_source_dir = self.selected_source_dir.clone().unwrap();
                        let selected_dist_dir = self
                            .selected_dest_dir
                            .clone()
                            .unwrap_or(self.selected_source_dir.clone().unwrap());
                        let output_format = self.output_format.clone();
                        let dds_format = self.dds_format.clone();

                        let tx = self.tx.clone();

                        thread::spawn(move || {
                            let result = convert(
                                files,
                                selected_source_dir,
                                selected_dist_dir,
                                output_format,
                                dds_format,
                            );
                            tx.send(result).expect("failed to send result");
                        });
                    }
                },
            );

            #[cfg(debug_assertions)]
            if ui.button("debug").clicked() {
                self.is_debug_panel_open = true;
            }

            ui.separator();

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.vertical(|ui| {
                    self.table_ui(ui);
                });

                // show image when column selected
                if self.selected_row_index > -1 {
                    let files = self.files.clone().unwrap();
                    let index = self.selected_row_index.clone();
                    let current_file = &files[index as usize];

                    ui.add(egui::Image::new(format!("file://{current_file}")).rounding(5.0));
                }
            });

            // Bottom panel
            egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("v{}", &env!("CARGO_PKG_VERSION")));
                    egui::warn_if_debug_build(ui);
                    ui.hyperlink_to("Project homepage", env!("CARGO_PKG_HOMEPAGE"));
                });
            });

            // get signal when convert is done
            let result = self.rx.try_recv();
            match result {
                Ok(success) => {
                    if success {
                        self.is_convert_success = Some(true);
                    } else {
                        self.is_convert_success = Some(false);
                    }
                }
                Err(e) => {
                    if e == TryRecvError::Disconnected {
                        error!("{:?}", e);
                    }
                }
            }

            if self.set_window_open_flag {
                self.is_window_open = false;
                self.set_window_open_flag = false;
            }
        });

        let screen_rect = ctx.screen_rect();
        let center_pos = Pos2::new(screen_rect.width() / 2.0, screen_rect.height() / 2.0);

        // convert panel
        egui::Window::new("")
            .open(&mut self.is_window_open)
            .title_bar(false)
            .default_pos(center_pos)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    if let Some(is_convert_success) = self.is_convert_success {
                        if is_convert_success {
                            ui.label(RichText::new("Success!").color(Color32::GREEN));
                        } else {
                            ui.label(RichText::new("Failed!").color(Color32::RED));
                        }
                        if ui.button("Done!").clicked() {
                            self.set_window_open_flag = true;
                        }
                    } else {
                        ui.add(egui::Spinner::new().size(60.0));
                        ui.label("converting...");
                    }
                });
            });

        // debug panel
        #[cfg(debug_assertions)]
        egui::Window::new("Debug Panel")
            .open(&mut self.is_debug_panel_open)
            .default_width(280.0)
            .show(ctx, |ui| {
                egui::Grid::new("my_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        let mut selected_source_dir =
                            self.selected_source_dir.clone().unwrap_or("".to_string());
                        ui.label("selected source dir");
                        ui.add(
                            egui::TextEdit::singleline(&mut selected_source_dir)
                                .hint_text("Write something here"),
                        );
                        ui.end_row();

                        let mut selected_dist_dir =
                            self.selected_source_dir.clone().unwrap_or("".to_string());
                        ui.label("selected dist dir");
                        ui.add(
                            egui::TextEdit::singleline(&mut selected_dist_dir)
                                .hint_text("Write something here"),
                        );
                        ui.end_row();

                        let files_size = self.files.clone().unwrap_or(vec![]).len();
                        ui.label("files size");
                        ui.label(format!("{}", files_size));
                        ui.end_row();

                        ui.label("is window open");
                        ui.end_row();

                        ui.label("is convert success");
                        ui.end_row();
                    });
            });
    }
}

impl ImageConverterApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let (tx, rx) = channel();
        Self {
            selected_source_dir: None,
            selected_dest_dir: None,
            files: None,
            output_format: ImageFormatEnum::PNG,
            dds_format: image_dds::ImageFormat::BC1RgbaUnorm,
            selected_row_index: -1,
            is_window_open: false,
            is_convert_success: None,
            is_debug_panel_open: false,
            set_window_open_flag: false,
            tx,
            rx,
        }
    }

    fn table_ui(&mut self, ui: &mut egui::Ui) {
        use egui_extras::{Column, TableBuilder};

        let available_height = ui.available_height();
        let table = TableBuilder::new(ui)
            .column(Column::auto().resizable(true))
            .column(Column::remainder())
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height)
            .sense(egui::Sense::click());

        if self.files.is_some() {
            let files = self.files.clone().unwrap();

            table
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Images");
                    });
                })
                .body(|mut body| {
                    for file_path in files {
                        body.row(30.0, |mut row| {
                            let row_index = row.index();

                            row.set_selected(self.selected_row_index == row_index as i8);
                            row.col(|ui| {
                                ui.label(file_path);
                            });

                            self.toggle_row_selection(row.index(), &row.response());
                        });
                    }
                });
        }
    }

    fn toggle_row_selection(&mut self, row_index: usize, row_response: &egui::Response) {
        if row_response.clicked() {
            if self.selected_row_index == -1 || row_index != self.selected_row_index as usize {
                self.selected_row_index = row_index as i8;
            } else {
                self.selected_row_index = -1;
            }
        }
    }
}
