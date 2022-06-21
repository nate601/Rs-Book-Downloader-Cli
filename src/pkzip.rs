use std::collections::VecDeque;
use std::io::prelude::*;
use std::io::Cursor;
use std::io::Seek;
use std::io::SeekFrom;
use std::ops::BitAnd;

#[derive(Debug)]
pub struct PkZip
{
    pub local_file_headers: Vec<LocalFileHeader>,
    pub central_directory_headers: Vec<CentralDirectoryHeader>,
    pub end_of_central_directory_record: EndOfCentralDirectoryRecord,
    pub file_bytes: Vec<u8>,
}

impl PkZip
{
    pub fn data_is_pkzip(file: &[u8]) -> bool
    {
        let pkzip_magic_numbers: &[u8] = &[0x50, 0x4b, 0x03, 0x04];
        file.starts_with(pkzip_magic_numbers)
    }
    pub fn get_files(&self) -> Vec<PkZipFile>
    {
        let ret_val: &mut Vec<PkZipFile> = &mut Vec::new();
        let cursor = &mut Cursor::new(&self.file_bytes);
        for (cdh, lfh) in self
            .central_directory_headers
            .iter()
            .zip(self.local_file_headers.to_vec())
        {
            cursor
                .seek(std::io::SeekFrom::Start(lfh.end_position))
                .unwrap();
            let compressed_size = &mut u32::from_le_bytes(cdh.compressed_size);
            let buf: &mut Vec<u8> = &mut Vec::with_capacity(*compressed_size as usize);
            buf.resize(*compressed_size as usize, 0u8);
            assert!(*compressed_size as usize == buf.len());
            cursor.read_exact(buf).unwrap();
            let k = PkZipFile {
                file_name: String::from_utf8(cdh.file_name.to_vec()).unwrap(),
                compressed_size: *compressed_size,
                compression_method: get_compression_method(u16::from_le_bytes(
                    cdh.compression_method,
                ))
                .unwrap(),
                uncompressed_size: u32::from_le_bytes(cdh.uncompressed_size),
                crc_32: u32::from_le_bytes(cdh.crc_32),
                compressed_data: buf.to_vec(),
            };
            ret_val.push(k);
        }
        ret_val.to_vec()
    }
    pub fn new(file_bytes: &[u8]) -> Self
    {
        let _pkzip_magic_numbers: &[u8] = &[0x50, 0x4b, 0x03, 0x04];
        let pkzip_central_file_header_signature: &[u8] = &[0x50, 0x4b, 0x01, 0x02];
        let pkzip_end_of_central_directory_signature: &[u8] = &[0x50, 0x4b, 0x05, 0x06];
        let _pkzip_local_file_header_signature: &[u8] = &[0x50, 0x4b, 0x03, 0x04];

        let mut cursor = Cursor::new(file_bytes);
        // file_bytes.iter().position()
        //
        //

        //Find and seek to ECDR Signature
        let position_of_ecdr = file_bytes
            .windows(pkzip_end_of_central_directory_signature.len())
            .position(|x| x == pkzip_end_of_central_directory_signature)
            .unwrap() as u64;
        println!("posiiton of ECDR sig {:#?}", position_of_ecdr);
        cursor
            .seek(std::io::SeekFrom::Start(position_of_ecdr))
            .unwrap();
        cursor.seek(std::io::SeekFrom::Current(4)).unwrap();
        let number_of_this_disk: &mut [u8; 2] = &mut [0; 2];
        let number_of_disk_with_start_of_central_directory: &mut [u8; 2] = &mut [0; 2];
        let total_number_of_entries_in_central_directory_on_current_disk: &mut [u8; 2] =
            &mut [0; 2];
        let total_number_of_entries_in_central_directory: &mut [u8; 2] = &mut [0; 2];
        let size_of_central_directory: &mut [u8; 4] = &mut [0; 4];
        let offset_of_start_of_central_directory_with_respect_to_starting_disk_number: &mut [u8;
                 4] = &mut [0; 4];
        let zip_file_comment_length: &mut [u8; 2] = &mut [0; 2];

        cursor.read_exact(number_of_this_disk).unwrap();
        cursor
            .read_exact(number_of_disk_with_start_of_central_directory)
            .unwrap();
        cursor
            .read_exact(total_number_of_entries_in_central_directory_on_current_disk)
            .unwrap();
        cursor
            .read_exact(total_number_of_entries_in_central_directory)
            .unwrap();
        cursor.read_exact(size_of_central_directory).unwrap();
        cursor
            .read_exact(offset_of_start_of_central_directory_with_respect_to_starting_disk_number)
            .unwrap();
        cursor.read_exact(zip_file_comment_length).unwrap();
        let zip_file_comment_length_value = u16::from_be_bytes(*zip_file_comment_length) as usize;
        let zip_file_comment = &mut Vec::with_capacity(zip_file_comment_length_value);
        zip_file_comment.resize(zip_file_comment_length_value, 0);
        cursor.read_exact(zip_file_comment).unwrap();

        //Fill ECDR
        let end_of_central_directory_record = EndOfCentralDirectoryRecord {
            number_of_this_disk: *number_of_this_disk,
            number_of_disk_with_start_of_central_directory:
                *number_of_disk_with_start_of_central_directory,
            total_number_of_entries_in_central_directory_on_current_disk:
                *total_number_of_entries_in_central_directory_on_current_disk,
            total_number_of_entries_in_central_directory:
                *total_number_of_entries_in_central_directory,
            size_of_central_directory: *size_of_central_directory,
            offset_of_start_of_central_directory_with_respect_to_starting_disk_number:
                *offset_of_start_of_central_directory_with_respect_to_starting_disk_number,
            zip_file_comment_length: *zip_file_comment_length,
            // Variable size
            zip_file_comment: zip_file_comment.to_vec(),
        };
        let central_directory_header: &mut Vec<CentralDirectoryHeader> = &mut Vec::with_capacity(
            u16::from_be_bytes(*total_number_of_entries_in_central_directory) as usize,
        );

        //Find and seek central directory records
        cursor
            .seek(std::io::SeekFrom::Start(u32::from_le_bytes(
                *offset_of_start_of_central_directory_with_respect_to_starting_disk_number,
            ) as u64))
            .unwrap();
        for _i in 0..u16::from_le_bytes(*total_number_of_entries_in_central_directory)
        {
            let start_posistion = cursor.position();
            let buf: &mut [u8; 4] = &mut [0; 4];
            cursor.read_exact(buf).unwrap();
            assert!(buf == pkzip_central_file_header_signature);

            let version_maker: &mut [u8; 2] = &mut [0; 2];
            let version_needed_to_extract: &mut [u8; 2] = &mut [0; 2];
            let general_purpose_bit_flag: &mut [u8; 2] = &mut [0; 2];
            let compression_method: &mut [u8; 2] = &mut [0; 2];
            let last_mod_file_time: &mut [u8; 2] = &mut [0; 2];
            let last_mod_file_date: &mut [u8; 2] = &mut [0; 2];
            let crc_32: &mut [u8; 4] = &mut [0; 4];
            let compressed_size: &mut [u8; 4] = &mut [0; 4];
            let uncompressed_size: &mut [u8; 4] = &mut [0; 4];
            let file_name_length: &mut [u8; 2] = &mut [0; 2];
            let extra_field_length: &mut [u8; 2] = &mut [0; 2];
            let file_comment_length: &mut [u8; 2] = &mut [0; 2];
            let disk_number_start: &mut [u8; 2] = &mut [0; 2];
            let internal_file_attributes: &mut [u8; 2] = &mut [0; 2];
            let external_file_attributes: &mut [u8; 4] = &mut [0; 4];
            let relative_offset_of_local_header: &mut [u8; 4] = &mut [0; 4];

            cursor.read_exact(version_maker).unwrap();
            cursor.read_exact(version_needed_to_extract).unwrap();
            cursor.read_exact(general_purpose_bit_flag).unwrap();
            cursor.read_exact(compression_method).unwrap();
            cursor.read_exact(last_mod_file_time).unwrap();
            cursor.read_exact(last_mod_file_date).unwrap();
            cursor.read_exact(crc_32).unwrap();
            cursor.read_exact(compressed_size).unwrap();
            cursor.read_exact(uncompressed_size).unwrap();
            cursor.read_exact(file_name_length).unwrap();
            cursor.read_exact(extra_field_length).unwrap();
            cursor.read_exact(file_comment_length).unwrap();
            cursor.read_exact(disk_number_start).unwrap();
            cursor.read_exact(internal_file_attributes).unwrap();
            cursor.read_exact(external_file_attributes).unwrap();
            cursor.read_exact(relative_offset_of_local_header).unwrap();

            let file_name_length_val = u16::from_le_bytes(*file_name_length) as usize;
            let file_name: &mut Vec<u8> =
                &mut Vec::with_capacity(u16::from_le_bytes(*file_name_length) as usize);
            file_name.resize(file_name_length_val, 0);

            let extra_field_length_val = u16::from_le_bytes(*extra_field_length) as usize;
            let extra_field: &mut Vec<u8> = &mut Vec::with_capacity(extra_field_length_val);
            extra_field.resize(extra_field_length_val, 0);

            let file_comment_length_val = u16::from_le_bytes(*file_comment_length) as usize;
            let file_comment: &mut Vec<u8> = &mut Vec::with_capacity(file_comment_length_val);
            file_comment.resize(file_comment_length_val, 0);

            cursor.read_exact(file_name).unwrap();
            cursor.read_exact(extra_field).unwrap();
            cursor.read_exact(file_comment).unwrap();
            let end_position = cursor.position();
            //Fill CDH
            let header = CentralDirectoryHeader {
                start_position: start_posistion,
                version_maker: *version_maker,
                version_needed_to_extract: *version_needed_to_extract,
                general_purpose_bit_flag: *general_purpose_bit_flag,
                compression_method: *compression_method,
                last_mod_file_time: *last_mod_file_time,
                last_mod_file_date: *last_mod_file_date,
                crc_32: *crc_32,
                compressed_size: *compressed_size,
                uncompressed_size: *uncompressed_size,
                file_name_length: *file_name_length,
                extra_field_length: *extra_field_length,
                file_comment_length: *file_comment_length,
                disk_number_start: *disk_number_start,
                internal_file_attributes: *internal_file_attributes,
                external_file_attributes: *external_file_attributes,
                relative_offset_of_local_header: *relative_offset_of_local_header,

                // Variable size
                file_name: file_name.to_vec(),
                extra_field: extra_field.to_vec(),
                file_comment: file_comment.to_vec(),
                end_position,
            };
            central_directory_header.push(header);
        }
        //Fill file headers
        let local_file_headers: &mut Vec<LocalFileHeader> =
            &mut Vec::with_capacity(central_directory_header.len());
        for x in central_directory_header.to_vec()
        {
            let offset = u32::from_le_bytes(x.relative_offset_of_local_header) as u64;
            cursor.seek(std::io::SeekFrom::Start(offset)).unwrap();

            let version_needed_to_extract: &mut [u8; 2] = &mut [0; 2];
            let general_purpose_bit_flag: &mut [u8; 2] = &mut [0; 2];
            let compression_method: &mut [u8; 2] = &mut [0; 2];
            let last_mod_file_time: &mut [u8; 2] = &mut [0; 2];
            let last_mod_file_date: &mut [u8; 2] = &mut [0; 2];
            let crc_32: &mut [u8; 4] = &mut [0; 4];
            let compressed_size: &mut [u8; 4] = &mut [0; 4];
            let uncompressed_size: &mut [u8; 4] = &mut [0; 4];
            let file_name_length: &mut [u8; 2] = &mut [0; 2];
            let extra_field_length: &mut [u8; 2] = &mut [0; 2];

            let buf = &mut [0u8; 4];

            cursor.read_exact(buf).unwrap();
            cursor.read_exact(version_needed_to_extract).unwrap();
            cursor.read_exact(general_purpose_bit_flag).unwrap();
            cursor.read_exact(compression_method).unwrap();
            cursor.read_exact(last_mod_file_time).unwrap();
            cursor.read_exact(last_mod_file_date).unwrap();
            cursor.read_exact(crc_32).unwrap();
            cursor.read_exact(compressed_size).unwrap();
            cursor.read_exact(uncompressed_size).unwrap();
            cursor.read_exact(file_name_length).unwrap();
            cursor.read_exact(extra_field_length).unwrap();

            //Variable size
            // let file_name: Vec<u8>,
            let file_name_length_val = u16::from_le_bytes(*file_name_length) as usize;
            let file_name: &mut Vec<u8> = &mut Vec::with_capacity(file_name_length_val);
            file_name.resize(file_name_length_val, 0);
            cursor.read_exact(file_name).unwrap();
            // let extra_field: Vec<u8>,
            let extra_field_length_val = u16::from_le_bytes(*extra_field_length) as usize;
            let extra_field: &mut Vec<u8> = &mut Vec::with_capacity(extra_field_length_val);
            extra_field.resize(extra_field_length_val, 0);
            cursor.read_exact(extra_field).unwrap();
            let end_position = cursor.position();

            let header = LocalFileHeader {
                start_position: offset,
                version_needed_to_extract: *version_needed_to_extract,
                general_purpose_bit_flag: *general_purpose_bit_flag,
                compression_method: *compression_method,
                last_mod_file_time: *last_mod_file_time,
                last_mod_file_date: *last_mod_file_date,
                crc_32: *crc_32,
                compressed_size: *compressed_size,
                uncompressed_size: *uncompressed_size,
                file_name_length: *file_name_length,
                extra_field_length: *extra_field_length,
                file_name: file_name.to_vec(),
                extra_field: extra_field.to_vec(),
                end_position,
            };

            local_file_headers.push(header);
        }

        //combine and return struct

        PkZip {
            local_file_headers: local_file_headers.to_vec(),
            central_directory_headers: central_directory_header.to_vec(),
            end_of_central_directory_record,
            file_bytes: file_bytes.to_vec(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct PkZipFile
{
    pub file_name: String,
    pub compressed_size: u32,
    pub compression_method: CompressionMethod,
    pub uncompressed_size: u32,
    pub crc_32: u32,
    pub compressed_data: Vec<u8>,
}

#[derive(Debug)]
pub struct BitArray
{
    pub byte: u8,
    pub bits: Vec<bool>,
}

impl BitArray
{
    pub fn new(byte: u8) -> Self
    {
        let bits: &mut Vec<bool> = &mut Vec::with_capacity(8);
        let mut byte_moved = byte;
        for _x in 0..8
        {
            let lz = byte_moved.leading_zeros();
            if lz >= 1
            {
                bits.push(false);
            }
            else
            {
                bits.push(true);
            }
            byte_moved = byte_moved.rotate_left(1);
        }
        Self {
            byte,
            bits: bits.to_vec(),
        }
    }
    pub fn new_from_bits(bist: &[bool]) -> Self
    {
        let mut byte = 0x0u8;
        for i in 0..8
        {
            byte = byte.rotate_left(1);
            let v = bist.get(i);
            if v.is_some() && *v.unwrap()
            {
                byte += 1;
            }
        }
        Self::new(byte)
    }
    pub fn get_byte(&self) -> u8
    {
        let mut byte = 0x0u8;
        for i in 0..8
        {
            byte = byte.rotate_left(1);
            let v = self.bits.get(i);
            if v.is_some() && *v.unwrap()
            {
                byte += 1;
            }
        }
        byte
    }
    pub fn get_bits(source: Vec<u8>) -> Vec<bool>
    {
        let mut ret_val: Vec<bool> = Vec::new();
        for x in source
        {
            let ba = BitArray::new(x);
            for b in ba.bits
            {
                ret_val.push(b);
            }
        }
        ret_val
    }
    pub fn is_bit_set(source: u8, n: u8) -> bool
    {
        let temp: u8 = 1 << (n - 1);
        if source.bitand(temp) != 0
        {
            return true;
        }
        false
    }
    pub fn get_cursor(o: Vec<bool>) -> Cursor<Vec<u8>>
    {
        let mut k: Vec<u8> = Vec::new();
        for i in o
        {
            if i
            {
                k.push(1)
            }
            else
            {
                k.push(0)
            }
        }
        Cursor::new(k.to_vec())
    }
    pub fn get_bits_as_u8_vector(&self) -> Vec<u8>
    {
        let mut k: Vec<u8> = Vec::new();
        for i in self.bits.to_vec()
        {
            if i
            {
                k.push(1)
            }
            else
            {
                k.push(0)
            }
        }
        k
    }
    pub fn get_bytearray_vec_as_combined_u8_vec(o: &[Self]) -> Vec<u8>
    {
        let mut r: Vec<u8> = Vec::new();
        for i in o
        {
            for j in i.get_bits_as_u8_vector()
            {
                r.push(j);
            }
        }
        r
    }
}
#[derive(Debug)]
pub struct ByteStream
{
    data: Vec<BitArray>,
    dq: VecDeque<u8>,
}

impl ByteStream
{
    pub fn get_bit(&mut self) -> Result<bool, &'static str>
    {
        match self.dq.pop_front()
        {
            Some(val) => Ok(val == 1),
            None => Err("No more remaining bits"),
        }
    }
    pub fn get_number_from_arbitrary_bits(&mut self, number_of_bits: u8)
        -> Result<u8, &'static str>
    {
        let bits = self.get_bits(number_of_bits)?;
        let mut b = VecDeque::from_iter(bits);
        while b.len() < 8
        {
            b.push_front(false);
        }
        println!("{:#?}", b);
        let ba = BitArray::new_from_bits(&Vec::from(b)[0..=7]);
        println!("{:#?}", ba);
        Ok(ba.byte)
    }
    pub fn get_bits(&mut self, number_of_bits: u8) -> Result<Vec<bool>, &'static str>
    {
        let mut bits: Vec<bool> = Vec::new();
        for i in 0..number_of_bits
        {
            bits.push(self.get_bit()?);
        }
        Ok(bits)
    }
    pub fn get_byte(&mut self) -> Result<u8, &'static str>
    {
        self.get_number_from_arbitrary_bits(8)
    }
    pub fn skip_until_byte_aligned(&mut self) -> Result<(), &'static str>
    {
        loop
        {
            if self.dq.len() % 8 == 0
            {
                break;
            }
            self.get_bit()?;
        }
        Ok(())
    }
    pub fn is_at_end(&self) -> bool
    {
        self.dq.is_empty()
    }
    pub fn new(data: Vec<BitArray>) -> Self
    {
        Self {
            dq: VecDeque::from_iter(BitArray::get_bytearray_vec_as_combined_u8_vec(&data)),
            data,
        }
    }
    pub fn get_next_symbol(&mut self, tree: &HuffmanTree) -> u8
    {
        let mut cur_node = &tree.root_node;
        loop
        {
            if cur_node.right.is_some() || cur_node.left.is_some()
            {
                let b = self.get_bit().unwrap();
                if b
                {
                    //right
                    cur_node = cur_node.right.as_ref().unwrap();
                }
                else
                {
                    cur_node = cur_node.left.as_ref().unwrap();
                }
            }
            else
            {
                break;
            }
        }
        cur_node.value.unwrap()
    }
}

impl PkZipFile
{
    pub fn decompress(&self) -> Result<Vec<u8>, &'static str>
    {
        if self.uncompressed_size == 0
        {
            return Ok(vec![0]);
        }
        match self.compression_method
        {
            CompressionMethod::NoCompression => Ok(self.compressed_data.to_vec()),
            CompressionMethod::Deflated =>
            {
                let mut ret_val: Vec<u8> = Vec::new();
                let mut ret_cursor = Cursor::new(ret_val);
                let mut compressed_byte_arrays: Vec<BitArray> = Vec::new();
                for i in self.compressed_data.to_vec()
                {
                    compressed_byte_arrays.push(BitArray::new(i));
                }
                let mut byte_stream = ByteStream::new(compressed_byte_arrays);

                loop
                {
                    println!("New block!!######################");
                    let is_last_block = byte_stream.get_bit().unwrap();
                    let compression_type_indicator =
                        byte_stream.get_number_from_arbitrary_bits(2).unwrap();
                    let compression_type = get_deflate_compression_type(compression_type_indicator);
                    println!("Compression type: {:#?} ", compression_type);
                    match compression_type
                    {
                        DeflateCompressionType::Stored =>
                        {
                            byte_stream.skip_until_byte_aligned()?;
                            let mut len_buf: [u8;2] = [0u8;2];
                            len_buf[0] = byte_stream.get_byte().unwrap();
                            len_buf[1] = byte_stream.get_byte().unwrap();
                            let len = u16::from_be_bytes(len_buf);
                            for _i in 0..len
                            {
                                ret_cursor.write_all(&[byte_stream.get_byte()?]).unwrap();
                            }
                        }
                        DeflateCompressionType::FixedHuffman => todo!(),
                        DeflateCompressionType::DynamicHuffman => todo!(),
                        DeflateCompressionType::Reserved =>
                        {
                            return Err(
                                "Malformed zip file, DeflateCompresionType is of Reserved type.",
                            )
                        }
                    }

                    if is_last_block
                    {
                        println!("Last block");
                        break;
                    }
                }
                while !byte_stream.is_at_end()
                {
                    let val = byte_stream.get_bit().unwrap();
                    if val
                    {
                        print!("X");
                    }
                    else
                    {
                        print!("_");
                    }
                }
                todo!();
            }
            _ => Err("Unimplemented"),
        }
    }
}
#[derive(Debug)]
pub struct Node
{
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
    value: Option<u8>,
}

impl Node
{
    pub fn new() -> Self
    {
        Self {
            left: None,
            right: None,
            value: None,
        }
    }
}
#[derive(Debug)]
pub struct HuffmanTree
{
    root_node: Box<Node>,
}

impl HuffmanTree
{
    pub fn new() -> Self
    {
        Self {
            root_node: Box::new(Node::new()),
        }
    }
    pub fn insert(&mut self, address: u8, address_len: u8, value: u8)
    {
        let mut cur_node = &mut self.root_node;
        for i in (0..address_len).rev()
        {
            let b = address & (1 << i);
            if b == 1
            {
                //right
                if cur_node.right.is_none()
                {
                    let new_node = Box::new(Node::new());
                    cur_node.right = Some(new_node);
                }
                cur_node = cur_node.right.as_mut().unwrap();
            }
            else
            {
                //left
                if cur_node.left.is_none()
                {
                    let new_node = Box::new(Node::new());
                    cur_node.left = Some(new_node);
                }
                cur_node = cur_node.left.as_mut().unwrap();
            }
        }
        cur_node.value = Some(value);
    }
    pub fn get_value(self, address: u8, address_len: u8) -> Option<u8>
    {
        let mut cur_node = self.root_node;
        for i in (0..address_len).rev()
        {
            let b = address & (1 << i);
            if b == 1
            {
                cur_node = cur_node.right.unwrap();
            }
            else
            {
                cur_node = cur_node.left.unwrap();
            }
        }
        cur_node.value
    }
}
fn get_compression_method(method_identifier: u16) -> Result<CompressionMethod, &'static str>
{
    Ok(match method_identifier
    {
        0 => CompressionMethod::NoCompression,
        1 => CompressionMethod::Shrunk,
        2 => CompressionMethod::ReducedWithCompressionFactorOfOne,
        3 => CompressionMethod::ReducedWithCompressionFactorOfTwo,
        4 => CompressionMethod::ReducedWithCompressionFactorOfThree,
        5 => CompressionMethod::ReducedWithCompressionFactorOfFour,
        6 => CompressionMethod::Imploded,
        7 => CompressionMethod::Tokenizing,
        8 => CompressionMethod::Deflated,
        9 => CompressionMethod::Deflate64,
        10 => CompressionMethod::IbmTerse,
        11 => CompressionMethod::ReservedByPkWareOne,
        12 => CompressionMethod::Bzip2,
        13 => CompressionMethod::ReservedByPkWareTwo,
        14 => CompressionMethod::Lzma,
        15 => CompressionMethod::ReservedByPkWareThree,
        16 => CompressionMethod::IbmCmpSc,
        17 => CompressionMethod::ReservedByPkWareFour,
        18 => CompressionMethod::IbmTerseNew,
        19 => CompressionMethod::IbmLz77,
        20 => CompressionMethod::Deprecated,
        93 => CompressionMethod::Zstd,
        94 => CompressionMethod::Mp3,
        95 => CompressionMethod::Xz,
        96 => CompressionMethod::Jpeg,
        97 => CompressionMethod::WavPack,
        98 => CompressionMethod::Ppmd,
        99 => CompressionMethod::Aex,
        _ => return Err("Unknown compression type"),
    })
}
#[derive(Debug, Clone)]
pub enum DeflateCompressionType
{
    Stored,
    FixedHuffman,
    DynamicHuffman,
    Reserved,
}
fn get_deflate_compression_type(type_indicator: u8) -> DeflateCompressionType
{
    match type_indicator
    {
        0 => DeflateCompressionType::Stored,
        1 => DeflateCompressionType::FixedHuffman,
        2 => DeflateCompressionType::DynamicHuffman,
        3 => DeflateCompressionType::Reserved,
        //According to the spec, a type indicator of 3 should be treated as an error, so there is
        //no issue as treating an unhandled type indicator as if it were 3
        _ => DeflateCompressionType::Reserved,
    }
}
#[derive(Debug, Clone)]
pub enum CompressionMethod
{
    NoCompression,
    Shrunk,
    ReducedWithCompressionFactorOfOne,
    ReducedWithCompressionFactorOfTwo,
    ReducedWithCompressionFactorOfThree,
    ReducedWithCompressionFactorOfFour,
    Imploded,
    Tokenizing,
    Deflated,
    Deflate64,
    IbmTerse,
    ReservedByPkWareOne,
    Bzip2,
    ReservedByPkWareTwo,
    Lzma,
    ReservedByPkWareThree,
    IbmCmpSc,
    ReservedByPkWareFour,
    IbmTerseNew,
    IbmLz77,
    Deprecated,
    Zstd,
    Mp3,
    Xz,
    Jpeg,
    WavPack,
    Ppmd,
    Aex,
}
#[derive(Debug, Clone)]
pub struct LocalFileHeader
{
    pub start_position: u64,
    pub version_needed_to_extract: [u8; 2],
    pub general_purpose_bit_flag: [u8; 2],
    pub compression_method: [u8; 2],
    pub last_mod_file_time: [u8; 2],
    pub last_mod_file_date: [u8; 2],
    pub crc_32: [u8; 4],
    pub compressed_size: [u8; 4],
    pub uncompressed_size: [u8; 4],
    pub file_name_length: [u8; 2],
    pub extra_field_length: [u8; 2],
    //Variable size
    pub file_name: Vec<u8>,
    pub extra_field: Vec<u8>,
    pub end_position: u64,
}
#[derive(Debug, Clone)]
pub struct CentralDirectoryHeader
{
    pub start_position: u64,
    pub version_maker: [u8; 2],
    pub version_needed_to_extract: [u8; 2],
    pub general_purpose_bit_flag: [u8; 2],
    pub compression_method: [u8; 2],
    pub last_mod_file_time: [u8; 2],
    pub last_mod_file_date: [u8; 2],
    pub crc_32: [u8; 4],
    pub compressed_size: [u8; 4],
    pub uncompressed_size: [u8; 4],
    pub file_name_length: [u8; 2],
    pub extra_field_length: [u8; 2],
    pub file_comment_length: [u8; 2],
    pub disk_number_start: [u8; 2],
    pub internal_file_attributes: [u8; 2],
    pub external_file_attributes: [u8; 4],
    pub relative_offset_of_local_header: [u8; 4],
    // Variable size
    pub file_name: Vec<u8>,
    pub extra_field: Vec<u8>,
    pub file_comment: Vec<u8>,
    pub end_position: u64,
}
#[derive(Debug)]
pub struct EndOfCentralDirectoryRecord
{
    pub number_of_this_disk: [u8; 2],
    pub number_of_disk_with_start_of_central_directory: [u8; 2],
    pub total_number_of_entries_in_central_directory_on_current_disk: [u8; 2],
    pub total_number_of_entries_in_central_directory: [u8; 2],
    pub size_of_central_directory: [u8; 4],
    pub offset_of_start_of_central_directory_with_respect_to_starting_disk_number: [u8; 4],
    pub zip_file_comment_length: [u8; 2],
    // Variable size
    pub zip_file_comment: Vec<u8>,
}
