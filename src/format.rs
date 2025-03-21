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
    if let png::ColorType::GrayscaleAlpha = reader.info().color_type {
        bytes = bytes
            .chunks(2)
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect();
    }
    Ok(Image::new_from_bytes(width, height, bytes))
}

pub fn write_png<W>(image: &Image, output: W) -> Result<(), String>
where
    W: std::io::Write,
{
    let mut encoder = png::Encoder::new(output, image.width() as _, image.height() as _);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_compression(png::Compression::Best);
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
    let mut prev = Image::new(width, height);
    while let Some(frame) = reader.next_frame_info().map_err(|e| e.to_string())? {
        delay.push(frame.delay as u32 * 10);
        let frame_dispose = frame.dispose;
        let (frame_left, frame_top) = (frame.left as i32, frame.top as i32);
        let (frame_width, frame_height) = (frame.width as _, frame.height as _);
        // println!(
        //     "{}: {} {} {} {} {:?}",
        //     frames.len(),
        //     frame_left,
        //     frame_top,
        //     frame_width,
        //     frame_height,
        //     frame_dispose
        // );
        let mut diff = vec![0; reader.buffer_size()];
        reader
            .read_into_buffer(&mut diff)
            .map_err(|e| e.to_string())?;
        let diff = Image::new_from_bytes(frame_width, frame_height, diff);
        let mut image = prev.clone();
        for x in 0..frame_width as i32 {
            for y in 0..frame_height as i32 {
                let color = *diff.get_unchecked(x, y);
                if color.3 != 0 {
                    *image.get_unchecked_mut(frame_left + x, frame_top + y) = color;
                    if matches!(
                        frame_dispose,
                        gif::DisposalMethod::Keep | gif::DisposalMethod::Any
                    ) {
                        *prev.get_unchecked_mut(frame_left + x, frame_top + y) = color
                    }
                }
                if matches!(frame_dispose, gif::DisposalMethod::Background) {
                    *prev.get_unchecked_mut(frame_left + x, frame_top + y) = (0, 0, 0, 0).into()
                }
            }
        }
        frames.push(image);
    }
    Ok((frames, delay))
}

struct SubFrame {
    offset: (u16, u16),
    data: Grid<u8>,
}

fn to_indices_gif(
    frames: &Vec<&Image>,
) -> Option<(Vec<(SubFrame, gif::DisposalMethod)>, Vec<u8>, Option<u8>)> {
    // first build the global palette
    use std::collections::HashMap;
    let mut transparent = None;
    let mut palette = Vec::new();
    let mut colors = HashMap::new();
    for f in frames {
        for c in f.data() {
            if !colors.contains_key(c) {
                if c.3 > 0 && c.3 < 255 {
                    return None;
                }
                let idx = palette.len() / 3;
                if idx > 255 {
                    return None;
                }
                colors.insert(*c, idx as u8);
                palette.push(c.0);
                palette.push(c.1);
                palette.push(c.2);
                if c.3 == 0 {
                    transparent = Some(idx as _);
                }
            }
        }
    }
    let mut output = Vec::with_capacity(frames.len());
    let mut prev = &frames[0].blank();
    let mut bg_save;
    let ((mut x, mut y), mut diff) = frames[0].effective_area(|_, c| Some(c.3 > 0)).unwrap();
    for (i, this) in frames.iter().enumerate() {
        let mut bg = prev.clone();
        let next = frames.iter().cycle().nth(i + 1).unwrap();
        let next_next = frames.iter().cycle().nth(i + 2).unwrap();
        diff.blank().blit(&mut bg, x, y);
        let (new, dispose) = [
            (prev, gif::DisposalMethod::Previous),
            (&bg, gif::DisposalMethod::Background),
            (*this, gif::DisposalMethod::Keep),
        ]
        .iter()
        .filter_map(|(p, dispose)| {
            next.effective_area(|(x, y), next_c| {
                if next_c.3 > 0 {
                    Some((*p.get_unchecked(x, y) != next_c) || next_next.get_unchecked(x, y).3 == 0)
                } else if p.get_unchecked(x, y).3 == 0 {
                    Some(false)
                } else {
                    ((*dispose == gif::DisposalMethod::Previous && prev.get_unchecked(x, y).3 == 0)
                        || *dispose == gif::DisposalMethod::Background)
                        .then_some(false)
                }
            })
            .map(|a| (a, *dispose))
        })
        .min_by_key(|((_, diff), _)| diff.data().len())
        .unwrap();
        // gif doesn't allow empty frame
        if diff.width() == 0 || diff.height() == 0 {
            let index = *colors.get(this.get_unchecked(0, 0)).unwrap();
            output.push((
                SubFrame {
                    offset: (0, 0),
                    data: Grid::new_with(1, 1, index),
                },
                dispose,
            ));
        } else {
            output.push((
                SubFrame {
                    offset: (x as _, y as _),
                    data: diff.apply(|c| *colors.get(c).unwrap()),
                },
                dispose,
            ));
        }
        match dispose {
            gif::DisposalMethod::Keep => {
                prev = this;
            }
            gif::DisposalMethod::Background => {
                bg_save = bg;
                prev = &bg_save;
            }
            _ => (),
        }
        ((x, y), diff) = new;
    }
    Some((output, palette, transparent))
}

pub fn write_gif<W>(frames: Vec<&Image>, delay: Vec<u32>, output: W) -> Result<(), String>
where
    W: std::io::Write,
{
    let width = frames[0].width() as _;
    let height = frames[0].height() as _;
    if let Some((frames, palette, transparent)) = to_indices_gif(&frames) {
        let mut encoder =
            gif::Encoder::new(output, width, height, &palette[..]).map_err(|e| e.to_string())?;
        encoder
            .set_repeat(gif::Repeat::Infinite)
            .map_err(|e| e.to_string())?;
        for ((subframe, dispose), delay) in frames.into_iter().zip(delay) {
            let mut frame = gif::Frame::from_indexed_pixels(
                subframe.data.width() as _,
                subframe.data.height() as _,
                &subframe.data.data()[..],
                transparent,
            );
            frame.left = subframe.offset.0;
            frame.top = subframe.offset.1;
            frame.delay = (delay / 10) as _;
            frame.dispose = dispose;
            encoder.write_frame(&frame).map_err(|e| e.to_string())?;
        }
    } else {
        let mut encoder =
            gif::Encoder::new(output, width, height, &[]).map_err(|e| e.to_string())?;
        encoder
            .set_repeat(gif::Repeat::Infinite)
            .map_err(|e| e.to_string())?;
        // fallback to the simple and inefficient way: each frame overwrites all
        // also `from_rgba_speed` uses the NeuQuant quantization under the hood
        // so this could be *very* lossy
        for (f, d) in frames.iter().zip(delay) {
            let mut frame =
                gif::Frame::from_rgba_speed(width, height, &mut f.raw_data().to_vec(), 10);
            frame.delay = (d / 10) as _;
            frame.dispose = gif::DisposalMethod::Previous;
            encoder.write_frame(&frame).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn load_ase<R>(input: R) -> Result<(Vec<Vec<Image>>, Vec<bool>, Vec<u32>, Vec<Color>), String>
where
    R: std::io::Read,
{
    let ase = ase::Decoder::new(input)
        .map_err(|e| e.to_string())?
        .read()
        .map_err(|e| e.to_string())?;
    let (width, height) = (ase.width(), ase.height());
    let num_frames = ase.num_frames();
    let num_layers = ase.num_layers();
    let layers = (0..num_layers)
        .map(|l| {
            (0..num_frames)
                .map(|f| {
                    let mut im = Image::new(width as _, height as _);
                    if let Some((x, y, w, h, bytes)) = ase.get_cel_in_rgba(l, f) {
                        let f = Image::new_from_bytes(w as _, h as _, bytes);
                        f.blit(&mut im, x as _, y as _);
                    }
                    im
                })
                .collect()
        })
        .collect();
    let visibility = (0..num_layers)
        .map(|l| ase.get_visibility(l).unwrap())
        .collect();
    let delay = (0..num_frames)
        .map(|f| ase.get_duration(f).unwrap() as _)
        .collect();
    let palette = ase
        .palette
        .entries()
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if i == ase.transparent_index as _
                && matches!(ase.pixel_format, ase::PixelFormat::Indexed)
            {
                (0, 0, 0, 0).into()
            } else {
                c.rgba().into()
            }
        })
        .collect();
    Ok((layers, visibility, delay, palette))
}

use crate::grid::Grid;
fn to_indices_single(image: &Image, palette: &[Color]) -> Option<SubFrame> {
    let ((x, y), sub) = image.effective_area(|_, c| Some(c.3 != 0)).expect("");
    let (w, h) = (sub.width(), sub.height());
    let mut indices = Grid::new(w, h);
    for i in 0..w as i32 {
        for j in 0..h as i32 {
            let c = sub.get_unchecked(i, j);
            *indices.get_unchecked_mut(i, j) = palette.iter().position(|p| p == c)? as _;
        }
    }
    Some(SubFrame {
        offset: (x as _, y as _),
        data: indices,
    })
}
fn to_indices_vec(images: &Vec<&Image>, palette: &[Color]) -> Option<Vec<SubFrame>> {
    let mut indices = Vec::with_capacity(images.len());
    for im in images {
        indices.push(to_indices_single(im, palette)?);
    }
    Some(indices)
}
fn to_indices_vec_vec(images: &Vec<Vec<&Image>>, palette: &[Color]) -> Option<Vec<Vec<SubFrame>>> {
    let mut indices = Vec::with_capacity(images.len());
    for v in images {
        indices.push(to_indices_vec(v, palette)?);
    }
    Some(indices)
}

pub fn write_ase<W>(
    layers: Vec<Vec<&Image>>,
    visibility: Vec<bool>,
    delay: Vec<u32>,
    palette: Vec<Color>,
    output: W,
) -> Result<(), String>
where
    W: std::io::Write,
{
    let width = layers[0][0].width() as _;
    let height = layers[0][0].height() as _;
    let mut encoder = ase::Encoder::new(output);
    let layer_info = visibility
        .into_iter()
        .enumerate()
        .map(|(i, v)| (v, format!("Layer {i}")))
        .collect();
    let mut ase = ase::AseFile::new(width, height, layer_info, ase::PixelFormat::RGBA);
    if let Some(layers) = to_indices_vec_vec(&layers, &palette) {
        ase.pixel_format = ase::PixelFormat::Indexed;
        for (f, delay) in delay.iter().enumerate() {
            ase.new_frame(
                layers
                    .iter()
                    .map(|layer| {
                        let subframe = &layer[f];
                        let (x, y) = subframe.offset;
                        let (w, h) = (subframe.data.width() as _, subframe.data.height() as _);
                        (x as _, y as _, w, h, &layer[f].data.data()[..])
                    })
                    .collect(),
                *delay as _,
            )?;
        }
        for (i, c) in palette.into_iter().enumerate() {
            if c.3 == 0 {
                ase.transparent_index = i as _;
                ase.palette.push([0, 0, 0, 255].into());
            } else {
                ase.palette.push(<Color as Into<[u8; 4]>>::into(c).into());
            }
        }
    } else {
        for (f, delay) in delay.iter().enumerate() {
            ase.new_frame(
                layers
                    .iter()
                    .map(|layer| (0, 0, width, height, layer[f].raw_data()))
                    .collect(),
                *delay as _,
            )?;
        }
        for c in palette {
            ase.palette.push(<Color as Into<[u8; 4]>>::into(c).into());
        }
    }
    encoder.write(&ase).map_err(|e| e.to_string())?;
    Ok(())
}
