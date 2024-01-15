use byteorder::{LittleEndian, ReadBytesExt};
use std::any::Any;
use std::fs::File;
use std::io::{self, Read};

#[derive(Debug, Clone)]
enum FlagEnum {
    Header,
    Data,
}

impl FlagEnum {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(FlagEnum::Header),
            0xFF => Some(FlagEnum::Data),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum HeaderTypeEnum {
    Program,
    NumArray,
    CharArray,
    Bytes,
}

impl HeaderTypeEnum {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(HeaderTypeEnum::Program),
            0x01 => Some(HeaderTypeEnum::NumArray),
            0x02 => Some(HeaderTypeEnum::CharArray),
            0x03 => Some(HeaderTypeEnum::Bytes),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct ProgramParams {
    autostart_line: u16,
    len_program: u16,
}

impl ProgramParams {
    fn new<R: Read>(reader: &mut R) -> io::Result<Self> {
        Ok(ProgramParams {
            autostart_line: reader.read_u16::<LittleEndian>()?,
            len_program: reader.read_u16::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
struct BytesParams {
    start_address: u16,
    reserved: [u8; 2],
}

impl BytesParams {
    fn new<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut reserved = [0; 2];
        reader.read_exact(&mut reserved)?;
        Ok(BytesParams {
            start_address: reader.read_u16::<LittleEndian>()?,
            reserved,
        })
    }
}

#[derive(Debug)]
struct ArrayParams {
    reserved: u8,
    var_name: u8,
    reserved1: [u8; 2],
}

impl ArrayParams {
    fn new<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut reserved1 = [0; 2];
        reader.read_exact(&mut reserved1)?;
        if reserved1 != [0x00, 0x80] {
            // Handle validation error here if needed
        }
        Ok(ArrayParams {
            reserved: reader.read_u8()?,
            var_name: reader.read_u8()?,
            reserved1,
        })
    }
}

#[derive(Debug)]
enum BlockParams {
    Program(ProgramParams),
    Array(ArrayParams),
    Bytes(BytesParams),
}

#[derive(Debug)]
struct Header {
    header_type: HeaderTypeEnum,
    filename: [u8; 10],
    len_data: u16,
    params: Option<BlockParams>, // Use the common enum type here
    checksum: u8,
}

impl Header {
    fn new<R: Read>(reader: &mut R) -> io::Result<Self> {
        let header_type = HeaderTypeEnum::from_u8(reader.read_u8()?).ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid header type",
        ))?;

        let mut filename = [0; 10];
        reader.read_exact(&mut filename)?;

        let len_data = reader.read_u16::<LittleEndian>()?;

        let params = match header_type {
            HeaderTypeEnum::Program => {
                let program_params = ProgramParams::new(reader)?;
                Some(BlockParams::Program(program_params))
            }
            HeaderTypeEnum::NumArray | HeaderTypeEnum::CharArray => {
                let array_params = ArrayParams::new(reader)?;
                Some(BlockParams::Array(array_params))
            }
            HeaderTypeEnum::Bytes => {
                let bytes_params = BytesParams::new(reader)?;
                Some(BlockParams::Bytes(bytes_params))
            }
        };

        let checksum = reader.read_u8()?;

        Ok(Header {
            header_type,
            filename,
            len_data,
            params,
            checksum,
        })
    }
}

#[derive(Debug)]
struct Block {
    len_block: u16,
    flag: FlagEnum,
    header: Option<Header>,
    data: Option<Vec<u8>>,
    headerless_data: Option<Vec<u8>>,
}

impl Block {
    fn new<R: Read>(reader: &mut R) -> io::Result<Self> {
        let len_block = reader.read_u16::<LittleEndian>()?;
        let flag = FlagEnum::from_u8(reader.read_u8()?).ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid block flag",
        ))?;

        let flag_clone = flag.clone();

        let (header, data, headerless_data) = match (len_block, flag_clone) {
            (19, FlagEnum::Header) => {
                let header = Some(Header::new(reader)?);
                let data = Some(vec![0; header.as_ref().unwrap().len_data as usize + 4]);
                (header, data, None)
            }
            (19, FlagEnum::Data) => {
                let header = None;
                let data = Some(vec![0; len_block as usize - 1]);
                (header, data, None)
            }
            (_, FlagEnum::Data) => {
                let header = None;
                let data = None;
                let headerless_data = Some(vec![0; len_block as usize - 1]);
                (header, data, headerless_data)
            }
            (_, _) => {
                let header = None;
                let data = None;
                let headerless_data = None;
                (header, data, headerless_data)
            }
        };

        Ok(Block {
            len_block,
            flag,
            header,
            data,
            headerless_data,
        })
    }
}

fn main() -> io::Result<()> {
    let mut file = File::open("./Android1.tap")?;
    let mut tap_data = Vec::new();
    file.read_to_end(&mut tap_data)?;

    let mut i = 0;

    while i < tap_data.len() {
        let block = Block::new(&mut &tap_data[i..])?;
        i += block.len_block as usize;

        // Display the block and its contents
        println!("{:?}", block.type_id());
    }

    Ok(())
}
