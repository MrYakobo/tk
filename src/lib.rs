use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

pub fn parse_aspect(joined: &str) -> Option<f64> {
    let both: Vec<&str> = joined.split('x').collect();
    if let Ok(x) = both[0].parse::<u32>() {
        if let Ok(y) = both[1].parse::<u32>() {
            return Some(x as f64 / y as f64);
        }
    }
    None
}

impl Pixel {
    pub fn empty() -> Self {
        Pixel {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }
    pub fn from_buf(buf: &[u8]) -> Self {
        // use as many values as possible
        //0xFF for all three values, 0xFE for two vals, 0xFD for one val

        match buf.len() {
            1 => Pixel {
                r: buf[0],
                g: 0,
                b: 0,
                a: 0xFD,
            },
            2 => Pixel {
                r: buf[0],
                g: buf[1],
                b: 0,
                a: 0xFE,
            },
            3 => Pixel {
                r: buf[0],
                g: buf[1],
                b: buf[2],
                a: 0xFF,
            },
            _ => Pixel {
                r: 0,
                g: 0,
                b: 0,
                a: 0xFF,
            },
        }
    }

    pub fn to_buf(&self) -> Vec<u8> {
        vec![self.r, self.g, self.b, self.a]
    }
}

pub fn encode(bytes: Vec<u8>, aspect: f64) -> io::Result<Vec<u8>> {
    let mut pixels: Vec<Pixel> = bytes.chunks(3).map(|c| Pixel::from_buf(c)).collect();

    let n_pixels = pixels.len() as u32;

    let width = (n_pixels as f64 * aspect).sqrt().ceil() as u32;
    let height = (n_pixels as f64 / aspect).sqrt().ceil() as u32;

    if width == 0 || height == 0 {
        return Err(io::Error::from(io::ErrorKind::InvalidData));
    }

    // if not perfectly fitting into a rectangle, fill with transparent pixels
    while (pixels.len() as u32) < (width * height) {
        pixels.push(Pixel::empty());
    }

    // we want to flatten the Vec<[u8;4]> to Vec<u8>
    // png_data_buffer is what is later sent to the png-encoder.
    // it requires a Vector of [r,g,b,a] values.
    let png_data_buffer: Vec<u8> = pixels.iter().flat_map(|p: &Pixel| p.to_buf()).collect();

    let mut out_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out_bytes, width, height);
        encoder.set_color(png::ColorType::RGBA);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header()?;
        writer.write_image_data(&png_data_buffer)?; // write the png_data into the vector
    }

    Ok(out_bytes)
}

pub fn encode_path(file_path: &PathBuf, png_path: &PathBuf, dimensions: &str) -> io::Result<()> {
    let mut file = File::open(file_path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    let aspect = parse_aspect(dimensions).unwrap();

    let encoded = encode(bytes, aspect)?;

    let mut png_file = File::create(png_path)?;
    png_file.write_all(&encoded)?;

    Ok(())
}

pub fn decode(reader: &mut dyn Read) -> io::Result<Vec<u8>> {
    let decoder = png::Decoder::new(reader);
    let (info, mut reader) = decoder.read_info()?;
    let mut pixels = vec![0; info.buffer_size()];
    reader.next_frame(&mut pixels)?;

    // also, any trailing 0-bytes are vasked
    let bytes: Vec<u8> = pixels
        .chunks(4)
        .flat_map(|chunk: &[u8]| {
            // if alpha byte is zero, then this is a filler pixel that we can skip
            // how many bytes are missing?
            match 0xFF - chunk[3] {
                0 => &chunk[0..3],
                1 => &chunk[0..2],
                2 => &chunk[0..1],
                _ => &[][..], // empty slice
            }
        })
        .cloned()
        .collect();

    // now we have our original bytes
    Ok(bytes)
}

pub fn decode_path(png_path: &PathBuf, output_path: &PathBuf) -> io::Result<()> {
    let mut f = File::open(png_path)?;
    let bytes = decode(&mut f)?;
    let mut file = File::create(output_path)?;
    file.write_all(&bytes)?;
    Ok(())
}
