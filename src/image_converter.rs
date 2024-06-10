use std::path::Path;

use crate::app::ImageFormatEnum;
use image_dds::ddsfile;
use log::{debug, error, info};
use pathdiff::diff_paths;
use rayon::prelude::*;

pub fn convert(
    files: Vec<String>,
    source_dir: String,
    output_path: String,
    output_format: ImageFormatEnum,
    dds_format: image_dds::ImageFormat,
    use_sequential_convert: bool,
) -> bool {
    return match output_format {
        ImageFormatEnum::DDS => {
            let convert_result = if use_sequential_convert {
                images_to_dds_sequential(files, source_dir, output_path, dds_format)
            } else {
                images_to_dds_parallel(files, source_dir, output_path, dds_format)
            };
            if convert_result.is_err() {
                error!("convert failed {:?}", convert_result.err());
                return false;
            }
            true
        }
        ImageFormatEnum::PNG => {
            let convert_result = if use_sequential_convert {
                dds_to_images_sequential(files, source_dir, output_path, output_format)
            } else {
                dds_to_images_parallel(files, source_dir, output_path, output_format)
            };
            if convert_result.is_err() {
                error!("convert failed {:?}", convert_result.err());
                return false;
            }
            true
        }
    };
}

fn images_to_dds_sequential(
    files: Vec<String>,
    source_dir: String,
    output_path: String,
    dds_format: image_dds::ImageFormat,
) -> anyhow::Result<()> {
    info!("converting start");

    // to prevent processing dds->dds, filter out dds files from files
    let files: Vec<String> = files
        .iter()
        .filter(|path| !path.ends_with(".dds"))
        .cloned()
        .collect();
    let files_size = files.len();

    for path_string in files {
        let cloned_path = path_string.clone();
        // in order to support processing directory recursive, get diff between current path and source path
        let mut source_relative_path =
            diff_paths(Path::new(&cloned_path), Path::new(&source_dir.clone())).ok_or_else(
                || anyhow::anyhow!("Failed to compute relative path for {}", cloned_path),
            )?;

        let image = image_dds::image::open(Path::new(&path_string))?;
        let rgba_image = image.to_rgba8();

        let dds = image_dds::dds_from_image(
            &rgba_image,
            dds_format,
            image_dds::Quality::Fast,
            image_dds::Mipmaps::GeneratedAutomatic,
        )?;

        source_relative_path.set_extension("dds");
        let output_path = Path::new(&output_path).join(source_relative_path);
        let mut writer = std::io::BufWriter::new(std::fs::File::create(&output_path)?);
        dds.write(&mut writer)?;
        debug!("{:?} created", output_path);
    }
    info!("converting ended. total files: {}", files_size);

    Ok(())
}

fn images_to_dds_parallel(
    files: Vec<String>,
    source_dir: String,
    output_path: String,
    dds_format: image_dds::ImageFormat,
) -> anyhow::Result<()> {
    info!("converting start");

    // to prevent processing dds->dds, filter out dds files from files
    let files: Vec<String> = files
        .into_iter()
        .filter(|path| !path.ends_with(".dds"))
        .collect();
    let files_size = files.len();

    // Use par_chunks to process files in parallel batches
    files
        .par_chunks(5)
        .try_for_each(|chunk| -> anyhow::Result<()> {
            chunk
                .iter()
                .try_for_each(|path_string| -> anyhow::Result<()> {
                    let cloned_path = path_string.clone();
                    let mut source_relative_path = diff_paths(
                        Path::new(&cloned_path),
                        Path::new(&source_dir),
                    )
                    .ok_or_else(|| {
                        anyhow::anyhow!("Failed to compute relative path for {}", cloned_path)
                    })?;

                    let image = image_dds::image::open(Path::new(&path_string))?;
                    let rgba_image = image.to_rgba8();

                    let dds = image_dds::dds_from_image(
                        &rgba_image,
                        dds_format,
                        image_dds::Quality::Fast,
                        image_dds::Mipmaps::GeneratedAutomatic,
                    )?;

                    source_relative_path.set_extension("dds");
                    let output_path = Path::new(&output_path).join(source_relative_path);
                    let mut writer = std::io::BufWriter::new(std::fs::File::create(&output_path)?);
                    dds.write(&mut writer)?;
                    debug!("{:?} created", output_path);

                    Ok(())
                })
        })?;
    info!("converting ended. total files: {}", files_size);

    Ok(())
}

fn dds_to_images_sequential(
    files: Vec<String>,
    source_dir: String,
    output_path: String,
    output_format: ImageFormatEnum,
) -> anyhow::Result<()> {
    info!("converting start");

    // to prevent processing dds->dds, filter out dds files from files
    let sss: &str = output_format.into();
    let files: Vec<String> = files
        .iter()
        .filter(|path| !path.ends_with(sss))
        .cloned()
        .collect();
    let files_size = files.len();

    for path_string in files {
        let cloned_path = path_string.clone();
        // in order to support processing directory recursive, get diff between current path and source path
        let mut source_relative_path =
            diff_paths(Path::new(&cloned_path), Path::new(&source_dir.clone())).ok_or_else(
                || anyhow::anyhow!("Failed to compute relative path for {}", cloned_path),
            )?;

        let mut reader = std::fs::File::open(path_string)?;
        let dds = ddsfile::Dds::read(&mut reader).unwrap();

        let image = image_dds::image_from_dds(&dds, 0)?;

        let sss: &str = output_format.into();
        source_relative_path.set_extension(sss);
        let output_path = Path::new(&output_path).join(source_relative_path);

        image.save(&output_path)?;
        debug!("{:?} created", output_path);
    }
    info!("converting ended. total files: {}", files_size);

    Ok(())
}

fn dds_to_images_parallel(
    files: Vec<String>,
    source_dir: String,
    output_path: String,
    output_format: ImageFormatEnum,
) -> anyhow::Result<()> {
    info!("converting start");

    // to prevent processing dds->dds, filter out dds files from files
    let sss: &str = output_format.into();
    let files: Vec<String> = files
        .iter()
        .filter(|path| !path.ends_with(sss))
        .cloned()
        .collect();
    let files_size = files.len();

    files
        .par_chunks(5)
        .try_for_each(|chunk| -> anyhow::Result<()> {
            chunk
                .iter()
                .try_for_each(|path_string| -> anyhow::Result<()> {
                    let cloned_path = path_string.clone();
                    // in order to support processing directory recursive, get diff between current path and source path
                    let mut source_relative_path =
                        diff_paths(Path::new(&cloned_path), Path::new(&source_dir.clone()))
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "Failed to compute relative path for {}",
                                    cloned_path
                                )
                            })?;

                    let mut reader = std::fs::File::open(path_string)?;
                    let dds = ddsfile::Dds::read(&mut reader).unwrap();

                    let image = image_dds::image_from_dds(&dds, 0)?;

                    let sss: &str = output_format.into();
                    source_relative_path.set_extension(sss);
                    let output_path = Path::new(&output_path).join(source_relative_path);

                    image.save(&output_path)?;
                    debug!("{:?} created", output_path);

                    Ok(())
                })
        })?;
    info!("converting ended. total files: {}", files_size);

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_images_to_dds() {
        let files = vec![
            "./test_images/o-a_base.png".to_string(),
            "./test_images/sub/o-a_base.png".to_string(),
        ];
        let source_dir = "./test_images".to_string();
        let output_path = "./test_images".to_string();
        let dds_format = image_dds::ImageFormat::BC1RgbaUnorm;

        let convert_result = images_to_dds_sequential(files, source_dir, output_path, dds_format);

        assert!(convert_result.is_ok());

        assert!(Path::new("./test_images/o-a_base.dds").exists());
        assert!(Path::new("./test_images/sub/o-a_base.dds").exists());

        fs::remove_file("./test_images/o-a_base.dds").unwrap();
        fs::remove_file("./test_images/sub/o-a_base.dds").unwrap();
    }

    #[test]
    fn test_dds_to_images() {
        let files = vec![
            "./test_images/o-a_base2.dds".to_string(),
            "./test_images/sub/o-a_base2.dds".to_string(),
        ];
        let source_dir = "./test_images".to_string();
        let output_path = "./test_images".to_string();
        let output_format = ImageFormatEnum::PNG;

        let convert_result =
            dds_to_images_sequential(files, source_dir, output_path, output_format);

        assert!(convert_result.is_ok());

        assert!(Path::new("./test_images/o-a_base2.png").exists());
        assert!(Path::new("./test_images/sub/o-a_base2.png").exists());

        fs::remove_file("./test_images/o-a_base2.png").unwrap();
        fs::remove_file("./test_images/sub/o-a_base2.png").unwrap();
    }

    #[test]
    fn test_compare() {
        use std::time::Instant;

        let files = vec![
            "C:\\Users\\fhzot\\Desktop\\test\\o-a_base.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-a_nrm.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-a_spec.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-b_base.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-b_nrm.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-b_spec.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-c_base.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-c_nrm.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-c_spec.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-d_base.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-d_nrm.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-d_spec.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-e_base.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-e_nrm.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-e_spec.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-faczy66-a_base.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-faczy66-a_nrm.png".to_string(),
            "C:\\Users\\fhzot\\Desktop\\test\\o-faczy66-a_spec.png".to_string(),
        ];
        let files_clone = files.clone();

        let source_dir = "./test_images".to_string();
        let source_dir_clone = source_dir.clone();
        let output_path = "./test_images".to_string();
        let output_path_clone = output_path.clone();
        let dds_format = image_dds::ImageFormat::BC1RgbaUnorm;

        let start = Instant::now();
        let convert_result = images_to_dds_sequential(files, source_dir, output_path, dds_format);
        let duration = start.elapsed();

        assert!(convert_result.is_ok());
        println!(
            "Time elapsed in images_to_dds_sequential() is: {:?}",
            duration
        );

        let start = Instant::now();
        let convert_result =
            images_to_dds_parallel(files_clone, source_dir_clone, output_path_clone, dds_format);
        let duration = start.elapsed();

        assert!(convert_result.is_ok());
        println!(
            "Time elapsed in images_to_dds_parallel() is: {:?}",
            duration
        );
    }
}
