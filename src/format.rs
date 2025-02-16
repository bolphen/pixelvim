use crate::color::Color;
use crate::image::Image;

pub fn load_png<R>(input: R) -> Result<Image, String>
where
    R: std::io::Read,
{
    let mut decoder = png::Decoder::new(input);
    decoder.set_transformations(png::Transformations::ALPHA);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let (width, height) = (reader.info().width as _, reader.info().height as _);
    let mut bytes = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut bytes).map_err(|e| e.to_string())?;
    Ok(Image::new_from_bytes(width, height, bytes))
}

pub fn write_png<W>(image: &Image, output: W) -> Result<(), String>
where
    W: std::io::Write,
{
    let mut encoder = png::Encoder::new(output, image.width() as _, image.height() as _);
    encoder.set_color(png::ColorType::Rgba);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(image.raw_data())
        .map_err(|e| e.to_string())?;
    drop(writer);
    Ok(())
}

pub fn load_gif<R>(input: R) -> Result<(Vec<Image>, Vec<u32>), String>
where
    R: std::io::Read,
{
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut reader = decoder.read_info(input).map_err(|e| e.to_string())?;
    let (width, height) = (reader.width() as _, reader.height() as _);
    let mut frames = Vec::new();
    let mut delay = Vec::new();
    let mut prev = Image::new_with(width, height, Color(0, 0, 0, 0));
    let mut dispose = gif::DisposalMethod::Any;
    while let Some(frame) = reader.next_frame_info().map_err(|e| e.to_string())? {
        delay.push(frame.delay as u32 * 10);
        let frame_dispose = frame.dispose;
        let (frame_left, frame_top) = (frame.left as i32, frame.top as i32);
        let (frame_width, frame_height) = (frame.width as _, frame.height as _);
        let mut diff = vec![0; reader.buffer_size()];
        reader
            .read_into_buffer(&mut diff)
            .map_err(|e| e.to_string())?;
        let diff = Image::new_from_bytes(frame_width, frame_height, diff);
        let mut image = prev.clone();
        for x in 0..frame_width as i32 {
            for y in 0..frame_height as i32 {
                let color = *diff.get_unchecked(x, y);
                if color.3 != 0 || matches!(dispose, gif::DisposalMethod::Background) {
                    *image.get_unchecked_mut(frame_left + x, frame_top + y) = color;
                    if color.3 != 0
                        && matches!(
                            frame_dispose,
                            gif::DisposalMethod::Keep | gif::DisposalMethod::Any
                        )
                    {
                        *prev.get_unchecked_mut(frame_left + x, frame_top + y) = color
                    }
                }
            }
        }
        frames.push(image);
        dispose = frame_dispose;
    }
    Ok((frames, delay))
}

pub fn write_gif<W>(frames: Vec<&Image>, delay: Vec<u32>, output: W) -> Result<(), String>
where
    W: std::io::Write,
{
    let width = frames[0].width() as _;
    let height = frames[0].height() as _;
    let mut encoder = gif::Encoder::new(output, width, height, &[]).map_err(|e| e.to_string())?;
    encoder
        .set_repeat(gif::Repeat::Infinite)
        .map_err(|e| e.to_string())?;
    for (f, d) in frames.iter().zip(delay) {
        // the simple and inefficient way: each frame overwrites all
        // TODO make better use of gif's encoding capability
        // also `from_rgba_speed` uses the NeuQuant quantization under the hood
        // so this is *very* lossy
        let mut frame = gif::Frame::from_rgba_speed(width, height, &mut f.raw_data().to_vec(), 10);
        frame.delay = (d / 10) as _;
        frame.dispose = gif::DisposalMethod::Previous;
        encoder.write_frame(&frame).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn load_ase<R>(input: R) -> Result<(Vec<Vec<Image>>, Vec<bool>, Vec<u32>), String>
where
    R: std::io::Read,
{
    let ase = asefile::AsepriteFile::read(input).map_err(|e| e.to_string())?;
    let (width, height) = (ase.width(), ase.height());
    let num_frames = ase.num_frames();
    let layers = ase
        .layers()
        .map(|l| {
            (0..num_frames)
                .map(|j| {
                    let pixels = l.frame(j).image().into_vec();
                    Image::new_from_bytes(width, height, pixels)
                })
                .collect()
        })
        .collect();
    let visibility = ase.layers().map(|l| l.is_visible()).collect();
    let delay = (0..num_frames).map(|j| ase.frame(j).duration()).collect();
    Ok((layers, visibility, delay))
}
