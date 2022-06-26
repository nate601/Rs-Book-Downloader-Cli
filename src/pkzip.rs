use std::collections::HashMap;
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
            eprintln!("lfh.end_position = {:?}", lfh.end_position);
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
            bits: bits.to_vec(),
        }
    }
    pub fn new_from_word(byte: u16) -> Self
    {
        let bits: &mut Vec<bool> = &mut Vec::with_capacity(8);
        let mut byte_moved = byte;
        for _x in 0..u16::BITS
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
            bits: bits.to_vec(),
        }
    }
    pub fn new_from_bits(bist: &[bool]) -> Self
    {
        let mut byte = 0x0u8;
        for i in 0..bist.len()
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
    pub fn get_word(&self) -> u16
    {
        let mut byte = 0x0u16;
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
    pub fn get_string(&self) -> String
    {
        let mut retVal = String::new();
        retVal.push_str("0b");
        for i in &self.bits
        {
            if *i
            {
                retVal.push('1');
            }
            else
            {
                retVal.push('0');
            }
        }
        retVal
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

    pub fn get_flipped(&self) -> BitArray
    {
        let k: Vec<bool> = self.bits.to_owned().into_iter().rev().collect();
        Self::new_from_bits(&k)
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
    pub fn read_bit(&mut self) -> Result<bool, &'static str>
    {
        match self.dq.pop_front()
        {
            Some(val) => Ok(val == 1),
            None => Err("No more remaining bits"),
        }
    }
    pub fn read_number_from_arbitrary_bits(
        &mut self,
        number_of_bits: u16,
    ) -> Result<u16, &'static str>
    {
        let bits = self.read_bits(number_of_bits)?;
        let mut b = VecDeque::from_iter(bits);
        while b.len() < 16
        {
            b.push_front(false);
        }
        let ba = BitArray::new_from_bits(&Vec::from(b));
        Ok(ba.get_word())
    }
    pub fn read_bits(&mut self, number_of_bits: u16) -> Result<Vec<bool>, &'static str>
    {
        let mut bits: VecDeque<bool> = VecDeque::new();
        for i in 0..number_of_bits
        {
            bits.push_front(self.read_bit()?);
        }
        Ok(bits.into_iter().collect())
    }
    pub fn read_byte(&mut self) -> Result<u8, &'static str>
    {
        // Ok(self.get_number_from_arbitrary_bits(8).unwrap() as u8)
        let byte_bits = self.read_bits(8).unwrap();
        let ba1 = BitArray::new_from_bits(&byte_bits);
        Ok(ba1.get_byte())
    }
    pub fn skip_until_byte_aligned(&mut self) -> Result<(), &'static str>
    {
        loop
        {
            if self.dq.len() % 8 == 0
            {
                break;
            }
            self.read_bit()?;
        }
        Ok(())
    }
    pub fn is_at_end(&self) -> bool
    {
        self.dq.is_empty()
    }
    pub fn new(data: Vec<BitArray>) -> Self
    {
        //Data is packed from least significant bit of the byte to most significant bit
        let mut re_arranged_bytes: Vec<BitArray> = Vec::new();
        for i in data
        {
            re_arranged_bytes.push(i.get_flipped());
            // re_arranged_bytes.push(i);
        }
        Self {
            // dq: VecDeque::from_iter(BitArray::get_bytearray_vec_as_combined_u8_vec(&data)),
            dq: VecDeque::from_iter(BitArray::get_bytearray_vec_as_combined_u8_vec(
                &re_arranged_bytes,
            )),
            data: re_arranged_bytes,
        }
    }
    pub fn read_next_symbol(&mut self, tree: &HuffmanTree) -> u16
    {
        let mut cur_node = &tree.root_node;
        //TODO: remove debug value here
        let mut code_so_far = String::new();
        loop
        {
            if cur_node._right.is_some() || cur_node._left.is_some()
            {
                let b = self.read_bit().unwrap();
                if b
                {
                    //right
                    code_so_far.push('1');
                    cur_node = cur_node._right.as_ref().unwrap_or_else(|| panic!("Node has no child node {:#?}.  the current direction is {} (1=right). Code so far: {}  ",
                        cur_node, b, code_so_far));
                }
                else
                {
                    code_so_far.push('0');
                    cur_node = cur_node._left.as_ref().unwrap();
                }
            }
            else
            {
                break;
            }
        }
        let val = cur_node._value.unwrap();
        println!("Symbol decoded: 0b{} = {}", code_so_far, val);

        val
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
            CompressionMethod::NoCompression => return Ok(self.compressed_data.to_vec()),
            CompressionMethod::Deflated =>
            {
                let ret_val: Vec<u8> = Vec::new();
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
                    let is_last_block = byte_stream.read_bit().unwrap();
                    let compression_type_indicator =
                        byte_stream.read_number_from_arbitrary_bits(2).unwrap();
                    let compression_type = get_deflate_compression_type(compression_type_indicator);
                    println!("Compression type: {:#?} ", compression_type);
                    match compression_type
                    {
                        DeflateCompressionType::Stored =>
                        {
                            extract_stored(&mut byte_stream, &mut ret_cursor);
                        }
                        DeflateCompressionType::FixedHuffman =>
                        {
                            extract_fixed_huffman(&mut byte_stream, &mut ret_cursor);
                        }
                        DeflateCompressionType::DynamicHuffman =>
                        {
                            extract_dynamic_huffman(&mut byte_stream, &mut ret_cursor);
                        }
                        DeflateCompressionType::Reserved =>
                        {
                            return Err(
                                "Malformed zip file, DeflateCompresionType is of Reserved type.",
                            )
                        }
                    };

                    if is_last_block
                    {
                        println!("Last block");
                        break;
                    }
                }
                return Ok(ret_cursor.into_inner());
            }
            _ => return Err("Unimplemented"),
        }
    }
}

fn extract_dynamic_huffman(byte_stream: &mut ByteStream, ret_cursor: &mut Cursor<Vec<u8>>)
{
    let hlit = byte_stream.read_number_from_arbitrary_bits(5).unwrap() + 257u16; // # of literal/length codes
    let hdist = byte_stream.read_number_from_arbitrary_bits(5).unwrap() + 1u16; // # of Distance Codes
    let hclen = byte_stream.read_number_from_arbitrary_bits(4).unwrap() + 4u16; // # of Code Length codes

    let mut unsorted_lengths = Vec::new();
    for i in 0..hclen
    {
        let length = byte_stream.read_number_from_arbitrary_bits(3).unwrap();
        unsorted_lengths.push(length);
    }
    let sort_order: &[u16; 19] = &[
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut sorted: [u16; 19] = [0u16; 19];
    for (i, v) in unsorted_lengths.iter().enumerate()
    {
        sorted[sort_order[i] as usize] = *v;
    }
    let code_lengths_values: [u16; 19] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
    ];
    let ht_of_code_lengths = HuffmanTree::construct_from_bitlengths(&code_lengths_values, &sorted);
    //read code length bit lengths
    let mut last_bit_length: u16 = 0;

    let mut literal_length_bitlengths: Vec<u16> = Vec::new();
    loop
    {
        if literal_length_bitlengths.len() >= hlit as usize
        {
            break;
        }
        let bit_length_value = byte_stream.read_next_symbol(&ht_of_code_lengths);
        if bit_length_value <= 15
        {
            //bit length literal
            literal_length_bitlengths.push(bit_length_value);
            last_bit_length = bit_length_value;
            continue;
        }
        if bit_length_value == 16
        {
            let number_of_times_to_repeat =
                byte_stream.read_number_from_arbitrary_bits(2).unwrap() + 3u16;
            for i in 0..number_of_times_to_repeat
            {
                literal_length_bitlengths.push(last_bit_length);
            }
            continue;
        }
        if bit_length_value == 17
        {
            let number_of_times_to_repeat =
                byte_stream.read_number_from_arbitrary_bits(3).unwrap() + 3u16;
            for i in 0..number_of_times_to_repeat
            {
                literal_length_bitlengths.push(0);
            }
            last_bit_length = 0;
            continue;
        }
        if bit_length_value == 18
        {
            let number_of_times_to_repeat =
                byte_stream.read_number_from_arbitrary_bits(7).unwrap() + 11u16;
            for i in 0..number_of_times_to_repeat
            {
                literal_length_bitlengths.push(0);
            }
            last_bit_length = 0;
            continue;
        }
    }

    let mut distance_ht_bit_lengths: Vec<u16> = Vec::new();

    loop
    {
        if distance_ht_bit_lengths.len() >= hdist as usize
        {
            break;
        }
        let bit_length_value = byte_stream.read_next_symbol(&ht_of_code_lengths);
        if bit_length_value <= 15
        {
            distance_ht_bit_lengths.push(bit_length_value);
            last_bit_length = bit_length_value;
            continue;
        }
        if bit_length_value == 16
        {
            let number_of_times_to_repeat =
                byte_stream.read_number_from_arbitrary_bits(2).unwrap() + 3u16;
            for i in 0..number_of_times_to_repeat
            {
                distance_ht_bit_lengths.push(last_bit_length);
            }
            continue;
        }
        if bit_length_value == 17
        {
            let number_of_times_to_repeat =
                byte_stream.read_number_from_arbitrary_bits(3).unwrap() + 3u16;
            for i in 0..number_of_times_to_repeat
            {
                distance_ht_bit_lengths.push(0);
            }
            last_bit_length = 0;
            continue;
        }
        if bit_length_value == 18
        {
            let number_of_times_to_repeat =
                byte_stream.read_number_from_arbitrary_bits(7).unwrap() + 11u16;
            for i in 0..number_of_times_to_repeat
            {
                distance_ht_bit_lengths.push(0);
            }
            last_bit_length = 0;
            continue;
        }
    }
    eprintln!(
        "literal_length_bitlengths = {:?}",
        literal_length_bitlengths
    );
    eprintln!("distance_ht_bit_lengths = {:?}", distance_ht_bit_lengths);

    let literal_length_values: [u16; 287] = (0u16..287u16).collect::<Vec<_>>().try_into().unwrap();
    let distance_values: [u16; 30] = (0u16..=29u16).collect::<Vec<_>>().try_into().unwrap();

    let literal_length_ht =
        HuffmanTree::construct_from_bitlengths(&literal_length_values, &literal_length_bitlengths);
    let distance_ht =
        HuffmanTree::construct_from_bitlengths(&distance_values, &distance_ht_bit_lengths);
    let (_, _, length_values, distance_valuess) = get_fixed_huffman_trees();
    let ret_val: Vec<u8> = Vec::new();
    let mut ret_cursor = Cursor::new(ret_val);
    extract_using_given_huffman_trees(
        byte_stream,
        literal_length_ht,
        length_values,
        distance_ht,
        distance_valuess,
        &mut ret_cursor,
    );
    let text = String::from_utf8(ret_cursor.into_inner()).unwrap();
    eprintln!("text = {:?}", text);
}

fn extract_stored(byte_stream: &mut ByteStream, ret_cursor: &mut Cursor<Vec<u8>>)
{
    byte_stream.skip_until_byte_aligned().unwrap();
    let mut len_buf: [u8; 2] = [0u8; 2];
    len_buf[0] = byte_stream.read_byte().unwrap();
    len_buf[1] = byte_stream.read_byte().unwrap();
    let len = u16::from_be_bytes(len_buf);
    for _i in 0..len
    {
        ret_cursor
            .write_all(&[byte_stream.read_byte().unwrap()])
            .unwrap();
    }
}

fn extract_fixed_huffman(byte_stream: &mut ByteStream, ret_cursor: &mut Cursor<Vec<u8>>)
{
    let (literal_length_ht, distance_ht, length_values, distance_values) =
        get_fixed_huffman_trees();
    extract_using_given_huffman_trees(
        byte_stream,
        literal_length_ht,
        length_values,
        distance_ht,
        distance_values,
        ret_cursor,
    );
    let text = String::from_utf8(ret_cursor.get_ref().to_vec());
    eprintln!("text = {:?}", text);
}

fn extract_using_given_huffman_trees(
    byte_stream: &mut ByteStream,
    literal_length_ht: HuffmanTree,
    length_values: Vec<u16>,
    distance_ht: HuffmanTree,
    distance_values: Vec<u16>,
    ret_cursor: &mut Cursor<Vec<u8>>,
)
{
    loop
    {
        //decode literal character from input stream
        let next_literal_or_length = byte_stream.read_next_symbol(&literal_length_ht);
        if next_literal_or_length <= 255
        {
            //copy character to output stream
            eprintln!("literal = {:?}", next_literal_or_length);
            ret_cursor
                .write_all(&[next_literal_or_length as u8])
                .unwrap();
        }
        else
        {
            //end of block
            if next_literal_or_length == 256
            {
                eprintln!(
                    "END OF BLOCK ######################### = {:?}",
                    next_literal_or_length
                );
                //break from loop
                break;
            }
            else
            {
                let length_code = next_literal_or_length;
                eprintln!("length_code = {:?}", length_code);
                let mut length_base = None;
                let mut length_number_of_extra_bits = None;
                for (i, x) in length_values.to_owned().into_iter().enumerate()
                {
                    if i % 3 == 0 && x == length_code
                    {
                        length_base = Some(length_values[i + 2]);
                        length_number_of_extra_bits = Some(length_values[i + 1]);
                        break;
                    }
                }
                if (length_base.is_none())
                {
                    println!("no length base detected for code of: {}", length_code);
                    eprintln!("length_values = {:?}", length_values);
                }
                if (length_number_of_extra_bits.is_none())
                {
                    println!("no length_extra_bits detected for code of: {}", length_code);
                    eprintln!("length_values = {:?}", length_values);
                }
                let length = length_base.unwrap()
                    + byte_stream
                        .read_number_from_arbitrary_bits(length_number_of_extra_bits.unwrap())
                        .unwrap() as u16;
                let distance_code = byte_stream.read_next_symbol(&distance_ht);
                let mut distance_base = None;
                let mut distance_number_of_extra_bits = None;
                for (i, x) in distance_values.to_owned().into_iter().enumerate()
                {
                    if i % 3 == 0 && x == distance_code
                    {
                        distance_number_of_extra_bits = Some(distance_values[i + 1]);
                        distance_base = Some(distance_values[i + 2]);
                    }
                }
                if (distance_base.is_none())
                {
                    println!("no distance base for code of: {}", distance_code);
                    eprintln!("distance_values = {:?}", distance_values);
                }
                if (distance_number_of_extra_bits.is_none())
                {
                    println!("no distance number of bits for code of: {}", distance_code);
                    eprintln!("distance_values = {:?}", distance_values);
                }
                let distance = distance_base.unwrap()
                    + byte_stream
                        .read_number_from_arbitrary_bits(distance_number_of_extra_bits.unwrap())
                        .unwrap() as u16;
                println!("Seek -{} and copy {} bits", distance, length);
                ret_cursor
                    .seek(SeekFrom::Current(distance as i64 * -1i64))
                    .unwrap();
                let mut copy_value: Vec<u8> = vec![0; length as usize];
                ret_cursor.read_exact(&mut copy_value).unwrap();
                ret_cursor.seek(SeekFrom::End(0)).unwrap();
                ret_cursor.write_all(&copy_value).unwrap();
            }
        }
    }
}

pub fn get_fixed_huffman_trees() -> (HuffmanTree, HuffmanTree, Vec<u16>, Vec<u16>)
{
    let length_values: Vec<u16> = vec![
        257, 0, 3, 258, 0, 4, 259, 0, 5, 260, 0, 6, 261, 0, 7, 262, 0, 8, 263, 0, 9, 264, 0, 10,
        265, 1, 11, 266, 1, 13, 267, 1, 15, 268, 1, 17, 269, 2, 19, 270, 2, 23, 271, 2, 27, 272, 2,
        31, 273, 3, 35, 274, 3, 43, 275, 3, 51, 276, 3, 59, 277, 4, 67, 278, 4, 83, 279, 4, 99,
        280, 4, 115, 281, 5, 131, 282, 5, 163, 283, 5, 195, 284, 5, 227, 285, 0, 258,
    ];
    let distance_values: Vec<u16> = vec![
        0, 0, 1, 1, 0, 2, 2, 0, 3, 3, 0, 4, 4, 1, 5, 5, 1, 7, 6, 2, 9, 8, 3, 17, 9, 3, 25, 10, 4,
        33, 11, 4, 49, 12, 5, 65, 13, 5, 97, 14, 6, 129, 16, 7, 257, 17, 7, 385, 18, 8, 513, 19, 8,
        769, 20, 9, 1025, 21, 9, 1537, 22, 10, 2049, 24, 11, 4097, 25, 11, 6145, 26, 12, 8193, 27,
        12, 12289, 28, 13, 16385, 29, 13, 24577, 15, 6, 193,
    ];
    let literal_length_ht_values: [u16; 288] =
        (0u16..=287u16).collect::<Vec<_>>().try_into().unwrap();
    let mut literal_length_ht_bit_length: [u16; 288] = [0; 288];
    for (i, ele) in literal_length_ht_values.into_iter().enumerate()
    {
        if ele <= 143
        {
            literal_length_ht_bit_length[i] = 8;
            continue;
        }
        if ele <= 255
        {
            literal_length_ht_bit_length[i] = 9;
            continue;
        }
        if ele <= 279
        {
            literal_length_ht_bit_length[i] = 7;
            continue;
        }
        if ele <= 287
        {
            literal_length_ht_bit_length[i] = 8;
            continue;
        }
    }
    // eprintln!("literal_length_ht_values = {:?}", literal_length_ht_values);
    // eprintln!(
    //     "literal_length_ht_bit_length = {:?}",
    //     literal_length_ht_bit_length
    // );
    let literal_length_ht = HuffmanTree::construct_from_bitlengths(
        &literal_length_ht_values,
        &literal_length_ht_bit_length,
    );
    let distance_ht_values: [u16; 32] = (0u16..=31).collect::<Vec<_>>().try_into().unwrap();
    let distance_ht_bit_lengths: [u16; 32] = [5; 32];
    let distance_ht =
        HuffmanTree::construct_from_bitlengths(&distance_ht_values, &distance_ht_bit_lengths);
    (
        literal_length_ht,
        distance_ht,
        length_values,
        distance_values,
    )
}
#[derive(Debug)]
pub struct Node
{
    _left: Option<Box<Node>>,
    _right: Option<Box<Node>>,
    _value: Option<u16>,
}

impl Node
{
    pub fn new() -> Self
    {
        Self {
            _left: None,
            _right: None,
            _value: None,
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
    pub fn insert(&mut self, address: u16, address_len: u16, value: u16)
    {
        let mut cur_node = &mut self.root_node;
        for i in (0..address_len).rev()
        {
            let b = address & (1 << i);
            if b != 0
            {
                //right
                if cur_node._right.is_none()
                {
                    let new_node = Box::new(Node::new());
                    cur_node._right = Some(new_node);
                }
                cur_node = cur_node._right.as_mut().unwrap();
            }
            else
            {
                //left
                if cur_node._left.is_none()
                {
                    let new_node = Box::new(Node::new());
                    cur_node._left = Some(new_node);
                }
                cur_node = cur_node._left.as_mut().unwrap();
            }
        }
        cur_node._value = Some(value);
    }
    pub fn get_value(self, address: u16, address_len: u8) -> Option<u16>
    {
        let mut cur_node = self.root_node;
        for i in (0..address_len).rev()
        {
            let b = address & (1 << i);
            if b != 0
            {
                cur_node = cur_node._right.unwrap();
            }
            else
            {
                cur_node = cur_node._left.unwrap();
            }
        }
        cur_node._value
    }
    pub fn construct_from_bitlengths(values: &[u16], bit_lengths: &[u16]) -> Self
    {
        let max_bit_length = bit_lengths.iter().max().unwrap();
        let mut count_for_each_bit_length: HashMap<u16, u16> = HashMap::new();
        for bit_length in bit_lengths
        {
            count_for_each_bit_length.insert(
                *bit_length,
                count_for_each_bit_length.get(bit_length).unwrap_or(&0u16) + 1,
            );
        }

        let mut next_address: HashMap<u16, u16> = HashMap::new();
        for i in 2..=*max_bit_length
        {
            let k: u16 = (next_address.get(&(i - 1)).unwrap_or(&0)
                + count_for_each_bit_length.get(&(i - 1)).unwrap_or(&0))
                << 1;
            next_address.insert(i, k);
        }
        next_address.insert(1, 0);
        let mut ht = Self::new();
        for (i, v) in values.iter().enumerate()
        {
            let bit_length = bit_lengths.get(i).unwrap_or(&0u16);
            if *bit_length == 0u16
            {
                continue;
            }
            let s = BitArray::new_from_word(*next_address.get(bit_length).unwrap()).get_string();
            let s_val = *next_address.get(bit_length).unwrap();
            // eprintln!(
            //    "value {} is given address {}, of bit_length {}",
            //    v, s, bit_length
            // );
            ht.insert(s_val, *bit_length, *v);
            next_address.insert(*bit_length, s_val + 1);
        }
        ht
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
fn get_deflate_compression_type(type_indicator: u16) -> DeflateCompressionType
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
