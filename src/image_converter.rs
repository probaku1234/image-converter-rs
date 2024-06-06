use std::path::Path;

use image_dds::ddsfile;
use pathdiff::diff_paths;

use crate::ImageFormatEnum;

pub fn convert(files: Vec<String>, source_dir: String, output_path: String, output_format: ImageFormatEnum, dds_format: image_dds::ImageFormat) -> bool {
    return match output_format {
        ImageFormatEnum::DDS => {
            images_to_dds(files, source_dir, output_path, dds_format).is_ok()
        }
        ImageFormatEnum::PNG => {
            dds_to_images(files, source_dir, output_path, output_format).is_ok()
        }
    };
}

fn images_to_dds(files: Vec<String>, source_dir: String, output_path: String, dds_format: image_dds::ImageFormat) -> anyhow::Result<()> {
    // to prevent processing dds->dds, filter out dds files from files
    let files: Vec<String> = files.iter().filter(|path| !path.ends_with(".dds")).cloned().collect();

    for path_string in files {
        let cloned_path = path_string.clone();
        // in order to support processing directory recursive, get diff between current path and source path
        let mut source_relative_path = diff_paths(Path::new(&cloned_path), Path::new(&source_dir.clone())).unwrap();
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
        let mut writer = std::io::BufWriter::new(std::fs::File::create(output_path)?);
        dds.write(&mut writer)?;
    };

    Ok(())
}

fn dds_to_images(files: Vec<String>, source_dir: String, output_path: String, output_format: ImageFormatEnum) -> anyhow::Result<()> {
    // to prevent processing dds->dds, filter out dds files from files
    let sss: &str = output_format.into();
    let files: Vec<String> = files.iter().filter(|path| !path.ends_with(sss)).cloned().collect();

    for path_string in files {
        let cloned_path = path_string.clone();
        // in order to support processing directory recursive, get diff between current path and source path
        let mut source_relative_path = diff_paths(Path::new(&cloned_path), Path::new(&source_dir.clone())).unwrap();
        
        let mut reader = std::fs::File::open(path_string)?;
        let dds = ddsfile::Dds::read(&mut reader).unwrap();
        
        let image = image_dds::image_from_dds(&dds, 0)?;

        let sss: &str = output_format.into();
        source_relative_path.set_extension(sss);
        let output_path = Path::new(&output_path).join(source_relative_path);
        
        image.save(output_path)?;
    }
    
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

        let convert_result = images_to_dds(
            files,
            source_dir,
            output_path,
            dds_format,
        );

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
        
        let convert_result = dds_to_images(
            files,
            source_dir,
            output_path,
            output_format,
        );

        assert!(convert_result.is_ok());

        assert!(Path::new("./test_images/o-a_base2.png").exists());
        assert!(Path::new("./test_images/sub/o-a_base2.png").exists());

        fs::remove_file("./test_images/o-a_base2.png").unwrap();
        fs::remove_file("./test_images/sub/o-a_base2.png").unwrap();
    }
}