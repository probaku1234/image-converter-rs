#![cfg_attr(
    not(debug_assertions),
    windows_subsystem = "windows"
)]

use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;

use eframe::egui;
use egui::{Color32, RichText};
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use log::error;
use log::LevelFilter;
use strum::{EnumString, IntoStaticStr};
use strum::IntoEnumIterator;
use walkdir::WalkDir;

use crate::image_converter::convert;

mod image_converter;

#[derive(Debug, PartialEq, Clone, EnumString, IntoStaticStr, Copy)]
pub enum ImageFormatEnum {
    #[strum(serialize = "png")]
    PNG,
    #[strum(serialize = "dds")]
    DDS,
}

// TODO: separate files
struct MyApp {
    selected_source_dir: Option<String>,
    selected_dest_dir: Option<String>,
    files: Option<Vec<String>>,
    output_format: ImageFormatEnum,
    dds_format: image_dds::ImageFormat,
    selected_row_index: i8,
    is_window_open: bool,
    is_convert_success: Option<bool>,
    tx: Sender<bool>,
    rx: Receiver<bool>,
}


impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        if self.is_window_open {
            egui::Window::new("Modal").open(&mut self.is_window_open).title_bar(false).show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(egui::Spinner::new().size(60.0));
                    ui.label("converting...");
                });
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!self.is_window_open);

            ui.horizontal(|ui| {
                if ui.button("Select folder").clicked() {
                    let files = rfd::FileDialog::new()
                        .set_directory("/")
                        .pick_folder();

                    if files.is_some() {
                        self.selected_source_dir = Some(files.unwrap().to_str().unwrap().to_string());
                        self.files = None;
                    }
                }

                ui.label(format!("Source folder: {:?}", self.selected_source_dir));
            });
            ui.horizontal(|ui| {
                if ui.button("Select folder").clicked() {
                    let files = rfd::FileDialog::new()
                        .set_directory("/")
                        .pick_folder();

                    if files.is_some() {
                        self.selected_dest_dir = Some(files.unwrap().to_str().unwrap().to_string());
                    }
                }

                ui.label(format!("Destination folder: {:?}", self.selected_dest_dir));
            });

            if self.selected_source_dir.is_some() && self.files.is_none() {
                let selected_source_dir = self.selected_source_dir.clone();

                self.files = Some(WalkDir::new(selected_source_dir.unwrap())
                    .into_iter()
                    .filter_map(|file| file.ok())
                    .filter(|file| file.metadata().unwrap().is_file())
                    .filter(|file| {
                        let array: [String; 3] = ["dds".parse().unwrap(), "png".parse().unwrap(), "jpg".parse().unwrap()];
                        array.contains(&file.path().extension().unwrap_or("None".as_ref()).to_str().unwrap().to_string())
                    })
                    .map(|file| String::from(file.path().to_str().unwrap()))
                    .collect());
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
                                ui.selectable_value(&mut self.dds_format, format, format.to_string());
                            }
                        });
                }
            });

            ui.add_enabled_ui(self.files.as_ref().map_or(false, |vec| !vec.is_empty()), |ui| {
                if ui.button("Convert").clicked() {
                    self.is_convert_success = None;
                    self.is_window_open = true;

                    let files = self.files.clone().unwrap();
                    let selected_source_dir = self.selected_source_dir.clone().unwrap();
                    let selected_dist_dir = self.selected_dest_dir.clone().unwrap_or(self.selected_source_dir.clone().unwrap());
                    let output_format = self.output_format.clone();
                    let dds_format = self.dds_format.clone();

                    let tx = self.tx.clone();

                    thread::spawn(move || {
                        let result = convert(files, selected_source_dir, selected_dist_dir, output_format, dds_format);
                        tx.send(result).expect("failed to send result");
                    });
                }
            });

            if self.is_convert_success.is_some() {
                let is_convert_success = self.is_convert_success.unwrap();

                if is_convert_success {
                    ui.label(RichText::new("Success!").color(Color32::GREEN));
                } else {
                    ui.label(RichText::new("Failed!").color(Color32::RED));
                }
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

                    ui.add(
                        egui::Image::new(format!("file://{current_file}"))
                            .rounding(5.0)
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
                    self.is_window_open = false;
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
        });
    }
}

impl MyApp {
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
            .sense(egui::Sense::click())
            ;


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

fn init_logging() -> anyhow::Result<()> {
    use directories::UserDirs;

    let mut appenders = vec![];

    let user_dir = UserDirs::new();
    if user_dir.is_some() {
        let user_dir = user_dir.unwrap();
        if let Some(document_path) = user_dir.document_dir() {
            let logfile = FileAppender::builder()
                .encoder(Box::new(PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S)} | {({l}):5.5} | {f}:{L} — {m}{n}")))
                .build(document_path.join(Path::new("image converter rs/output.log")))?;
            appenders.push(Appender::builder().build("logfile", Box::new(logfile)));
        }
    }

    let stderr = ConsoleAppender::builder().target(Target::Stderr).build();
    appenders.push(Appender::builder().build("stderr", Box::new(stderr)));

    let config = Config::builder()
        .appenders(appenders)
        .build(Root::builder()
            .appender("logfile")
            .appender("stderr")
            .build(LevelFilter::Debug))?;

    log4rs::init_config(config)?;

    Ok(())
}

fn main() -> eframe::Result<()> {
    init_logging().expect("initializing logging should be success");

    eframe::run_native(
        "Image Converter",
        eframe::NativeOptions::default(),
        Box::new(|ctx| Box::new(MyApp::new(ctx))),
    )
}
