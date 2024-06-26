use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;

use eframe::epaint::Color32;
use egui::{Align2, Pos2, Rect, RichText, Vec2};
use image::io::Reader;
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
    #[strum(serialize = "tga")]
    TGA,
    #[strum(serialize = "JPEG")]
    JPEG,
    #[strum(serialize = "JPG")]
    JPG,
}

pub(crate) struct ImageConverterApp {
    selected_source_dir: Option<String>,
    selected_dest_dir: Option<String>,
    files: Option<Vec<String>>,
    output_format: ImageFormatEnum,
    dds_format: image_dds::ImageFormat,
    selected_row_index: i8,
    is_window_open: bool,
    is_convert_success: Option<i8>,
    #[cfg(debug_assertions)]
    is_debug_panel_open: bool,
    set_window_open_flag: bool,
    use_sequential_convert: bool,
    tx: Sender<i8>,
    rx: Receiver<i8>,
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
                            let array: [String; 5] = [
                                "dds".parse().unwrap(),
                                "png".parse().unwrap(),
                                "jpg".parse().unwrap(),
                                "jpeg".parse().unwrap(),
                                "tga".parse().unwrap(),
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
                        ui.selectable_value(&mut self.output_format, ImageFormatEnum::JPEG, "JPEG");
                        ui.selectable_value(&mut self.output_format, ImageFormatEnum::JPG, "JPG");
                        ui.selectable_value(&mut self.output_format, ImageFormatEnum::TGA, "TGA");
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
                        let use_sequential_convert = self.use_sequential_convert.clone();

                        let tx = self.tx.clone();

                        thread::spawn(move || {
                            let result = convert(
                                files,
                                selected_source_dir,
                                selected_dist_dir,
                                output_format,
                                dds_format,
                                use_sequential_convert,
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
                let mut first_row_top_right_pos = Pos2::new(0.0, 0.0);
                ui.vertical(|ui| {
                    first_row_top_right_pos = self.table_ui(ui);
                });

                // show image when column selected
                if self.selected_row_index > -1 {
                    let files = self.files.clone().unwrap();
                    let index = self.selected_row_index.clone();
                    let current_file = &files[index as usize];

                    // in order to get top pos of table, subtract table header height from y pos
                    let dimension_label_pos =
                        Pos2::new(first_row_top_right_pos.x, first_row_top_right_pos.y - 20.0);
                    let image_dimension =
                        self.get_image_dimension(current_file).unwrap_or_else(|e| {
                            error!("failed to get image dimension {:?}", e);
                            (0, 0)
                        });

                    ui.add(egui::Image::new(format!("file://{current_file}")).rounding(5.0));
                    ui.put(
                        Rect::from_min_size(dimension_label_pos, Vec2::new(100.0, 20.0)),
                        egui::Label::new(format!("{} x {}", image_dimension.0, image_dimension.1)),
                    );
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
                    self.is_convert_success = Some(success);
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
                self.files = None; // set None to refresh files list when convert done
            }
        });

        let screen_rect = ctx.screen_rect();
        let center_pos = Pos2::new(screen_rect.width() / 2.0, screen_rect.height() / 2.0);

        // convert panel
        egui::Window::new("")
            .open(&mut self.is_window_open)
            .title_bar(false)
            .resizable(false)
            .default_pos(center_pos)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    if let Some(is_convert_success) = self.is_convert_success {
                        if is_convert_success >= 0 {
                            ui.label(RichText::new("Success!").color(Color32::GREEN).size(30.0));
                        } else {
                            ui.label(RichText::new("Failed!").color(Color32::RED).size(30.0));
                        }
                        if ui.button("Done!").clicked() {
                            self.set_window_open_flag = true;
                        }
                    } else {
                        ui.add(egui::Spinner::new().size(100.0));
                        ui.label(RichText::new("converting...").size(30.0));
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
                        ui.label(format!("{:?}", self.is_window_open));
                        ui.end_row();

                        ui.label("is convert success");
                        ui.label(format!("{:?}", self.is_convert_success));
                        ui.end_row();

                        ui.label("use sequential convert");
                        ui.checkbox(&mut self.use_sequential_convert, "");
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
            #[cfg(debug_assertions)]
            is_debug_panel_open: false,
            set_window_open_flag: false,
            use_sequential_convert: false,
            tx,
            rx,
        }
    }

    fn table_ui(&mut self, ui: &mut egui::Ui) -> Pos2 {
        use egui_extras::{Column, TableBuilder};
        let mut row_top_right_corner_pos = Pos2::new(0.0, 0.0);

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
                    for (i, file_path) in files.iter().enumerate() {
                        body.row(30.0, |mut row| {
                            let row_index = row.index();

                            row.set_selected(self.selected_row_index == row_index as i8);
                            row.col(|ui| {
                                ui.label(file_path);
                            });

                            self.toggle_row_selection(row.index(), &row.response());

                            // store first column's right top pos
                            if i == 0 {
                                row_top_right_corner_pos = row.response().rect.right_top()
                            }
                        });
                    }
                });

            let refresh_button_pos = Pos2::new(
                row_top_right_corner_pos.x - 30.0,
                row_top_right_corner_pos.y - 25.0,
            );

            if ui
                .put(
                    Rect::from_min_size(refresh_button_pos, Vec2::new(20.0, 20.0)),
                    egui::Button::new(
                        RichText::new(format!("{}", egui_phosphor::regular::ARROW_CLOCKWISE))
                            .size(20.0),
                    ),
                )
                .clicked()
            {
                // set to initial value to prevent render image
                self.selected_row_index = -1;
                self.files = None;
            }
        }

        row_top_right_corner_pos
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

    fn get_image_dimension(&self, path: &String) -> anyhow::Result<(u32, u32)> {
        let path = Path::new(path);
        let reader = Reader::open(path)?;
        let dimensions = reader.into_dimensions()?;

        Ok(dimensions)
    }
}
