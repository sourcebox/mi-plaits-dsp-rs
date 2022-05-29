//! Writer for WAV files

use std::fs::File;
use std::path::Path;

use mi_plaits_dsp::dsp::SAMPLE_RATE;

pub fn write(filename: &str, data: &[f32]) -> std::io::Result<()> {
    let path = format!("./out/{filename}");
    let path = Path::new(path.as_str());
    let parent = path.parent().unwrap();
    std::fs::create_dir_all(parent).ok();
    let mut file = File::create(path)?;
    let header = wav::Header::new(wav::WAV_FORMAT_IEEE_FLOAT, 1, SAMPLE_RATE as u32, 32);
    wav::write(header, &wav::BitDepth::from(Vec::from(data)), &mut file)?;
    Ok(())
}
