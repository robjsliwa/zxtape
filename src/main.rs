use byteorder::{LittleEndian, ReadBytesExt};
use std::env;
use std::fs::File;
use std::io::BufRead;
use std::io::{self, BufReader, Error, ErrorKind, Read};

#[derive(Debug, Clone, PartialEq)]
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
    fn from_bytes(reader: &mut BufReader<File>) -> io::Result<Self> {
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
    fn from_bytes(reader: &mut BufReader<File>) -> io::Result<Self> {
        let bytes_params = BytesParams {
            start_address: reader.read_u16::<LittleEndian>()?,
            reserved: [reader.read_u8()?, reader.read_u8()?],
        };
        // if !bytes_params.reserved.iter().all(|&x| x == 0) {
        //     return Err(Error::new(ErrorKind::InvalidData, "Invalid bytes params"));
        // }

        Ok(bytes_params)
    }
}

#[derive(Debug)]
struct ArrayParams {
    reserved: u8,
    var_name: u8,
    reserved1: [u8; 2],
}

impl ArrayParams {
    fn from_bytes(reader: &mut BufReader<File>) -> io::Result<Self> {
        let array_params = ArrayParams {
            reserved: reader.read_u8()?,
            var_name: reader.read_u8()?,
            reserved1: [reader.read_u8()?, reader.read_u8()?],
        };
        if !array_params.reserved1.iter().all(|&x| x == 0) {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid array params"));
        }

        Ok(array_params)
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
    params: Option<BlockParams>,
    checksum: u8,
}

impl Header {
    fn from_bytes(reader: &mut BufReader<File>) -> Result<Header, Error> {
        let header_type = HeaderTypeEnum::from_u8(reader.read_u8()?)
            .ok_or(Error::new(ErrorKind::InvalidData, "Invalid header type"))?;

        let mut filename = [0; 10];
        reader.read_exact(&mut filename)?;

        let len_data = reader.read_u16::<LittleEndian>()?;

        let params = match header_type {
            HeaderTypeEnum::Program => {
                Some(BlockParams::Program(ProgramParams::from_bytes(reader)?))
            }
            HeaderTypeEnum::NumArray | HeaderTypeEnum::CharArray => {
                Some(BlockParams::Array(ArrayParams::from_bytes(reader)?))
            }
            HeaderTypeEnum::Bytes => Some(BlockParams::Bytes(BytesParams::from_bytes(reader)?)),
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
    fn from_bytes(reader: &mut BufReader<File>) -> Result<Block, Error> {
        println!("CHECK0");
        let len_block = reader.read_u16::<LittleEndian>()?;
        let flag = FlagEnum::from_u8(reader.read_u8()?)
            .ok_or(Error::new(ErrorKind::InvalidData, "Invalid flag"))?;

        let mut header = None;
        let mut data = None;

        if len_block == 0x13 && flag == FlagEnum::Header {
            header = match flag {
                FlagEnum::Header => Some(Header::from_bytes(reader)?),
                FlagEnum::Data => None,
            };
        }

        if len_block == 0x13 {
            let mut block_data = vec![0; (header.as_ref().unwrap().len_data + 4) as usize];
            reader.read_exact(&mut block_data)?;
            data = Some(block_data);
        }

        let headerless_data = match flag {
            FlagEnum::Header => None,
            FlagEnum::Data => {
                let mut headerless_data = vec![0; (len_block - 1) as usize];
                reader.read_exact(&mut headerless_data)?;
                Some(headerless_data)
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
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err(Error::new(ErrorKind::InvalidInput, "No file name provided"));
    }

    let filename = &args[1];
    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);

    while !reader.fill_buf()?.is_empty() {
        let block = Block::from_bytes(&mut reader)?;
        println!("{:?}", block);
    }

    Ok(())
}
