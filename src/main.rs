#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;

use log::LevelFilter;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

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
            appenders.push(Appender::builder().build("logfile", Box::new(logfile)));
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

fn main() -> eframe::Result<()> {
    init_logging().expect("initializing logging should be success");

    eframe::run_native(
        "Image Converter",
        eframe::NativeOptions::default(),
        Box::new(|ctx| Box::new(app::ImageConverterApp::new(ctx))),
    )
}
