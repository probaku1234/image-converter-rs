#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::IconData;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use log::{error, LevelFilter};
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::filter::threshold::ThresholdFilter;

mod app;
mod image_converter;

fn init_logging() -> anyhow::Result<()> {
    use directories::UserDirs;

    let mut appenders = vec![];

    let user_dir = UserDirs::new();
    if user_dir.is_some() {
        let user_dir = user_dir.unwrap();
        if let Some(document_path) = user_dir.document_dir() {
            let logfile = FileAppender::builder()
                .encoder(Box::new(PatternEncoder::new(
                    "{d(%Y-%m-%d %H:%M:%S)} | {({l}):5.5} | {f}:{L} — {m}{n}",
                )))
                .build(document_path.join(Path::new("image converter rs/output.log")))?;
            appenders.push(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(LevelFilter::Info)))
                    .build("logfile", Box::new(logfile)),
            );
        }
    }

    let stderr = ConsoleAppender::builder().target(Target::Stderr).build();
    appenders.push(Appender::builder().build("stderr", Box::new(stderr)));

    let config = Config::builder().appenders(appenders).build(
        Root::builder()
            .appender("logfile")
            .appender("stderr")
            .build(LevelFilter::Debug),
    )?;

    log4rs::init_config(config)?;

    Ok(())
}

fn load_embedded_icon() -> Arc<IconData> {
    let icon_bytes = include_bytes!("../assets/logo.ico"); // use this for release build
    let img_result = image::load(Cursor::new(&icon_bytes[..]), image::ImageFormat::Ico);
    if img_result.is_err() {
        error!("failed to get icon: {:?}", img_result.err());
        return Arc::new(IconData::default());
    }
    let img = img_result.unwrap().into_rgba8();

    let (width, height) = img.dimensions();
    let rgba = img.into_raw();

    Arc::new(IconData {
        rgba,
        width,
        height,
    })
}
fn main() -> eframe::Result<()> {
    init_logging().unwrap_or_else(|e| {
        eprintln!("failed to init logging: {:?}", e);
    });

    let icon_data = load_embedded_icon();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_icon(icon_data),
        ..Default::default()
    };

    eframe::run_native(
        "Image Converter",
        options,
        Box::new(|ctx| {
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

            ctx.egui_ctx.set_fonts(fonts);
            Box::new(app::ImageConverterApp::new(ctx))
        }),
    )
}
