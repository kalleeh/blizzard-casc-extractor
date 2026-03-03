// HD ANIM format parser for StarCraft: Remastered
// Handles 4x HD and 2x HD animated sprites with multiple layers

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use image::RgbaImage;

const ANIM_MAGIC: u32 = 0x4D494E41; // "ANIM"

#[derive(Debug, Clone)]
pub struct HdAnimHeader {
    pub filetype: u32,
    pub version: u16,
    pub layers: u16,
    pub entries: u16,
    pub layer_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HdAnimImage {
    pub ptr: u32,
    pub size: u32,
    pub tex_width: u16,
    pub tex_height: u16,
}

#[derive(Debug, Clone)]
pub struct HdAnimEntry {
    pub frames: u16,
    pub grp_width: u16,
    pub grp_height: u16,
    pub frame_ptr: u32,
    pub images: Vec<HdAnimImage>,
}

#[derive(Debug, Clone)]
pub struct HdAnimFrame {
    pub x: u16,
    pub y: u16,
    pub x_offset: i16,
    pub y_offset: i16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug)]
pub struct HdAnimFile {
    pub header: HdAnimHeader,
    pub entry: HdAnimEntry,
    pub frames: Vec<HdAnimFrame>,
    pub layer_data: Vec<Vec<u8>>,
}

impl HdAnimFile {
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(data);
        
        let header = Self::parse_header(&mut cursor)?;
        
        if header.filetype != ANIM_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid ANIM magic: 0x{:08x}", header.filetype),
            ));
        }
        
        if header.version == 0x0101 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "SD ANIM format not supported",
            ));
        }
        
        let entry = Self::parse_entry(&mut cursor)?;
        
        cursor.seek(SeekFrom::Start(entry.frame_ptr as u64))?;
        let mut frames = Vec::with_capacity(entry.frames as usize);
        for _ in 0..entry.frames {
            frames.push(Self::parse_frame(&mut cursor)?);
        }
        
        // Extract layer DDS data
        let mut layer_data = Vec::new();
        for (i, img) in entry.images.iter().enumerate() {
            // Only extract actual layers (not empty slots)
            if i < header.layers as usize && img.size > 0 {
                cursor.seek(SeekFrom::Start(img.ptr as u64))?;
                let mut data = vec![0u8; img.size as usize];
                cursor.read_exact(&mut data)?;
                layer_data.push(data);
            } else if i < header.layers as usize {
                layer_data.push(Vec::new());
            }
        }
        
        Ok(HdAnimFile {
            header,
            entry,
            frames,
            layer_data,
        })
    }
    
    fn parse_header(cursor: &mut Cursor<&[u8]>) -> io::Result<HdAnimHeader> {
        let filetype = cursor.read_u32::<LittleEndian>()?;
        let version = cursor.read_u16::<LittleEndian>()?;
        let _unk2 = cursor.read_u16::<LittleEndian>()?;
        let layers = cursor.read_u16::<LittleEndian>()?;
        let entries = cursor.read_u16::<LittleEndian>()?;
        
        let mut layer_names = Vec::new();
        for _ in 0..10 {
            let mut name_bytes = [0u8; 32];
            cursor.read_exact(&mut name_bytes)?;
            let name = String::from_utf8_lossy(&name_bytes)
                .trim_end_matches('\0')
                .to_string();
            if !name.is_empty() {
                layer_names.push(name);
            }
        }
        
        Ok(HdAnimHeader {
            filetype,
            version,
            layers,
            entries,
            layer_names,
        })
    }
    
    fn parse_entry(cursor: &mut Cursor<&[u8]>) -> io::Result<HdAnimEntry> {
        let frames = cursor.read_u16::<LittleEndian>()?;
        let _unk2 = cursor.read_u16::<LittleEndian>()?;
        let grp_width = cursor.read_u16::<LittleEndian>()?;
        let grp_height = cursor.read_u16::<LittleEndian>()?;
        let frame_ptr = cursor.read_u32::<LittleEndian>()?;
        
        let mut images = Vec::new();
        for _ in 0..10 {
            images.push(HdAnimImage {
                ptr: cursor.read_u32::<LittleEndian>()?,
                size: cursor.read_u32::<LittleEndian>()?,
                tex_width: cursor.read_u16::<LittleEndian>()?,
                tex_height: cursor.read_u16::<LittleEndian>()?,
            });
        }
        
        Ok(HdAnimEntry {
            frames,
            grp_width,
            grp_height,
            frame_ptr,
            images,
        })
    }
    
    fn parse_frame(cursor: &mut Cursor<&[u8]>) -> io::Result<HdAnimFrame> {
        let x = cursor.read_u16::<LittleEndian>()?;
        let y = cursor.read_u16::<LittleEndian>()?;
        let x_offset = cursor.read_i16::<LittleEndian>()?;
        let y_offset = cursor.read_i16::<LittleEndian>()?;
        let width = cursor.read_u16::<LittleEndian>()?;
        let height = cursor.read_u16::<LittleEndian>()?;
        let _unk1 = cursor.read_u16::<LittleEndian>()?;
        let _unk2 = cursor.read_u16::<LittleEndian>()?;
        
        Ok(HdAnimFrame {
            x,
            y,
            x_offset,
            y_offset,
            width,
            height,
        })
    }
    
    pub fn get_diffuse_layer(&self) -> Option<&[u8]> {
        self.layer_data.first().filter(|d| !d.is_empty()).map(|d| d.as_slice())
    }

    pub fn get_team_color_layer(&self) -> Option<&[u8]> {
        self.layer_data.get(1).filter(|d| !d.is_empty()).map(|d| d.as_slice())
    }

    pub fn get_layer(&self, index: usize) -> Option<&[u8]> {
        self.layer_data.get(index).filter(|d| !d.is_empty()).map(|d| d.as_slice())
    }

    // -------------------------------------------------------------------------
    // Team-color mask extraction helpers
    // -------------------------------------------------------------------------

    /// Decode the diffuse layer (layer 0) to an RGBA image.
    ///
    /// Returns `None` if layer 0 is absent or empty.
    pub fn diffuse_image(&self) -> Option<io::Result<RgbaImage>> {
        let dds = self.get_diffuse_layer()?;
        Some(crate::dds_converter::dds_to_png(dds))
    }

    /// Build the team-color mask PNG from layer 1.
    ///
    /// Each output pixel: R=G=B=BT.601 luminance of the TC pixel, A=TC alpha.
    /// Returns `None` if layer 1 is absent or empty.
    pub fn team_color_mask_image(&self) -> Option<io::Result<RgbaImage>> {
        let tc_dds = self.get_team_color_layer()?;
        Some(crate::dds_converter::build_tc_mask_png(tc_dds))
    }

    /// Build a diffuse PNG with hue stripped from all TC-masked pixels.
    ///
    /// Where TC layer alpha > 0 the diffuse R/G/B are replaced by BT.601
    /// luminance, keeping diffuse alpha.  Requires both layer 0 and layer 1.
    /// Returns `None` if either layer is absent.
    pub fn diffuse_tc_stripped_image(&self) -> Option<io::Result<RgbaImage>> {
        let diffuse_dds = self.get_diffuse_layer()?;
        let tc_dds = self.get_team_color_layer()?;
        Some(crate::dds_converter::build_diffuse_tc_stripped_png(diffuse_dds, tc_dds))
    }
}
