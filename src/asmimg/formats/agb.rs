use asmimg::encoder::{IndexedGraphicsEncoder, DirectGraphicsEncoder};
use asmimg::tiles::TileChunkIterator;

use std::io;
use std::io::Write;
use image::{GenericImage, Primitive, Rgba, Pixel};

/// Encode a series of RGBA colors as palette data.
fn encode_palette<'a, I: Iterator, T: Primitive, W: Write + 'a>(w: &'a mut W, palette: I, use_alpha: bool) -> io::Result<()> where I: Iterator<Item=Rgba<T>> {
    let imgmax = T::max_value();
    let mut out: [u8; 2] = [0, 0];

    for rgba in palette {
        let r : u16 = (rgba[0].to_f32().unwrap() / imgmax.to_f32().unwrap() * 255f32) as u16;
        let g : u16 = (rgba[1].to_f32().unwrap() / imgmax.to_f32().unwrap() * 255f32) as u16;
        let b : u16 = (rgba[2].to_f32().unwrap() / imgmax.to_f32().unwrap() * 255f32) as u16;
        let a : u16 = match use_alpha {
            true => (rgba[3].to_f32().unwrap() / imgmax.to_f32().unwrap()) as u16,
            false => 0
        };
        
        let enc_color: u16 = a & 0x80 << 8 | b & 0xF8 << 7 | g & 0xF8 << 2 | r >> 3;
        
        out[0] = ((enc_color >> 0) & 0xFF) as u8;
        out[1] = ((enc_color >> 8) & 0xFF) as u8;
        w.write(&out)?;
    }

    Ok(())
}

struct ImageRgbaIterator<'a, I, P, S> where I: Iterator<Item=(u32, u32, P)> + 'a, P: Pixel<Subpixel=S> + 'a, S: Primitive + 'a {
    i: &'a mut I
}

impl<'a, I, P, S> ImageRgbaIterator<'a, I, P, S> where I: Iterator<Item=(u32, u32, P)> + 'a, P: Pixel<Subpixel=S> + 'a, S: Primitive + 'a {
    pub fn new(i: &'a mut I) -> ImageRgbaIterator<'a, I, P, S> {
        ImageRgbaIterator {
            i: i
        }
    }
}

impl<'a, I, P, S> Iterator for ImageRgbaIterator<'a, I, P, S> where I: Iterator<Item=(u32, u32, P)> + 'a, P: Pixel<Subpixel=S> + 'a, S: Primitive + 'a {
    type Item = Rgba<S>;
    
    fn next(&mut self) -> Option<Self::Item> {
        match self.i.next() {
            None => None,
            Some((_, _, p)) => Some(p.to_rgba())
        }
    }
}

/// Encoder for 4bpp tile patterns for the AGB platform.
pub struct AGB4Encoder<'a, W: Write + 'a> {
    w: &'a mut W,
}

impl<'a, W:Write + 'a> AGB4Encoder<'a, W> {
    pub fn new(write: &'a mut W) -> AGB4Encoder<'a, W> {
        AGB4Encoder {
            w: write
        }
    }
}

impl<'a, W:Write> IndexedGraphicsEncoder for AGB4Encoder<'a, W> {
    fn encode_indexes<P: Primitive>(&mut self, data: Vec<P>, width: u32, _height: u32) -> io::Result<()> {
        let mut out: [u8; 1] = [0];
        
        for tile in TileChunkIterator::new(data, 8, 8, width) {
            for byte in tile.chunks(2) {
                out[0] = byte[0].to_u8().unwrap() & 0x0F | (byte[1].to_u8().unwrap() & 0x0F) << 4;
                self.w.write(&out)?;
            }
        }
        
        Ok(())
    }
    
    fn encode_palette<T: Primitive>(&mut self, palette: Vec<Rgba<T>>) -> io::Result<()> {
        encode_palette(self.w, palette.into_iter(), false)
    }
    
    fn palette_maxcol(&self) -> u16 {
        15
    }
}

/// Encoder for 8bpp tile patterns for the AGB platform.
pub struct AGB8Encoder<'a, W: Write + 'a> {
    w: &'a mut W,
    tsize: u32
}

impl<'a, W:Write + 'a> AGB8Encoder<'a, W> {
    pub fn new_tiled(write: &'a mut W) -> AGB8Encoder<'a, W> {
        AGB8Encoder {
            w: write,
            tsize: 8
        }
    }
    
    pub fn new_chunky(write: &'a mut W) -> AGB8Encoder<'a, W> { 
        AGB8Encoder {
            w: write,
            tsize: 1
        }
    }
}

impl<'a, W:Write> IndexedGraphicsEncoder for AGB8Encoder<'a, W> {
    fn encode_indexes<P: Primitive>(&mut self, data: Vec<P>, width: u32, _height: u32) -> io::Result<()> {
        let mut out: [u8; 64] = [0; 64];
        let tsize = (self.tsize * self.tsize) as usize;
        
        for tile in TileChunkIterator::new(data, self.tsize, self.tsize, width) {
            for (i, byte) in tile.into_iter().enumerate() {
                out[i] = byte.to_u8().unwrap() & 0xFF;
            }
            
            self.w.write(&out[0 .. tsize])?;
        }
        
        Ok(())
    }
    
    fn encode_palette<T: Primitive>(&mut self, palette: Vec<Rgba<T>>) -> io::Result<()> {
        encode_palette(self.w, palette.into_iter(), false)
    }
    
    fn palette_maxcol(&self) -> u16 {
        255
    }
}

pub struct AGB16Encoder<'a, W: Write + 'a> {
    w: &'a mut W,
    allow_ntr_alpha: bool
}

impl<'a, W: Write + 'a> AGB16Encoder<'a, W> {
    pub fn new_agb(write: &'a mut W) -> AGB16Encoder<'a, W> {
        AGB16Encoder {
            w: write,
            allow_ntr_alpha: false
        }
    }
    
    pub fn new_ntr(write: &'a mut W) -> AGB16Encoder<'a, W> {
        AGB16Encoder {
            w: write,
            allow_ntr_alpha: true
        }
    }
}

impl<'a, W: Write> DirectGraphicsEncoder for AGB16Encoder<'a, W> {
    fn encode_colors<I, P, S>(&mut self, image: &I) -> io::Result<()> where I: GenericImage<Pixel=P>, P: Pixel<Subpixel=S> + 'static, S: Primitive + 'static {
        encode_palette(self.w, ImageRgbaIterator::new(&mut image.pixels()), self.allow_ntr_alpha)
    }
}

#[cfg(test)]
mod tests {
    extern crate num;
    
    use std::io::Cursor;
    use asmimg::encoder::IndexedGraphicsEncoder;
    use asmimg::formats::agb::{AGB4Encoder, AGB8Encoder};
    
    #[test]
    fn data4_encode() {
        let src = num::range(0, 64).collect();
        let mut test_out = Cursor::new(Vec::with_capacity(32));
        
        {
            let mut agb4 = AGB4Encoder::new(&mut test_out);

            agb4.encode_indexes(src, 8, 8).unwrap();
        }
        
        let valid_out : Vec<u8> = vec![0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE,
                                     0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE,
                                     0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE,
                                     0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE];
        
        assert_eq!(test_out.get_ref(), &valid_out)
    }
    
    #[test]
    fn data8t_encode() {
        let src = num::range(0, 64).collect();
        let mut test_out = Cursor::new(Vec::with_capacity(64));
        
        {
            let mut agb4 = AGB8Encoder::new_tiled(&mut test_out);

            agb4.encode_indexes(src, 8, 8).unwrap();
        }
        
        let valid_out : Vec<u8> = num::range(0, 64).collect();
        
        assert_eq!(test_out.get_ref(), &valid_out)
    }
    
    #[test]
    fn data8c_encode() {
        let src = num::range(0, 64).collect();
        let mut test_out = Cursor::new(Vec::with_capacity(64));
        
        {
            let mut agb4 = AGB8Encoder::new_chunky(&mut test_out);

            agb4.encode_indexes(src, 8, 8).unwrap();
        }
        
        let valid_out : Vec<u8> = num::range(0, 64).collect();
        
        assert_eq!(test_out.get_ref(), &valid_out)
    }
}