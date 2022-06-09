use irc_connection::*;
use irc_message::*;
use message_prefix::*;
use std::io::prelude::*;
use std::io::{stdin, BufReader, Cursor};
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;

mod irc_connection;
mod irc_message;
mod message_prefix;

fn main()
{
    //Connect to server
    let mut connex = IrcConnection::connect("127.0.0.1:6667").unwrap();

    let (tx, rx) = mpsc::channel();
    //start read loop thread
    // read_loop(&mut connex);
    let read_connex = connex.try_clone().unwrap();
    let _read_loop_handle = thread::spawn(move || {
        read_loop(read_connex, tx);
    });
    //login to #bookz channel
    connex.send_command_args("NICK", "rapere").unwrap();
    connex
        .send_command_multiple_args("USER", vec!["rapere", " ", "8", " ", "*", " :nathan"])
        .unwrap();
    connex.send_command_args("JOIN", "#bookz").unwrap();

    assert!(matches!(connex.status, ConnectionStatus::Connected));
    //Ask user for desired book
    let mut name = String::new();
    println!("Book Title?");
    stdin().read_line(&mut name).expect("Unable to read line");
    println!("Searching for {}", name);
    name = format!("{}{}", "@search ", &name);
    //Request search results from SearchBox
    connex
        .send_message("#bookz", &name)
        .expect("Unable to send message");
    //wait to receive DCC Send request for packlist
    let dcc_send_request = receive_new_dcc(&rx);
    let mut dcc_connex = DccConnection::connect(dcc_send_request).unwrap();
    //Respond to DCC request and read all
    let zipped_results_file_bytes = dcc_connex.get_all_bytes();
    
    println!("{:#?} ", zipped_results_file_bytes);

    //TODO:verify file we received was pkzip file
    if file_is_pkzip(&zipped_results_file_bytes)
    {
        println!("File is PKZIP!");
    }
    else
    {
        panic!("Non PKZIP file sent when expected PKZIP {:#?}", zipped_results_file_bytes)
    }


    //TODO:unzip file received thru DCC request
    PkZip::new(&zipped_results_file_bytes);


    //TODO: parse the txt file from Searchbot
    
    //TODO:Present in a table the choices to the user

    //TODO:Once user has selected choice, send a new message in the IRC channel

    //TODO:wait for DCC request then save file
    
    


}

#[derive(Debug)]
struct PkZip {
    local_file_headers: Vec<LocalFileHeader>,
    central_directory_header: Vec<CentralDirectoryHeader>,
    end_of_central_directory_record: EndOfCentralDirectoryRecord,
    file_bytes: Vec<u8>
}

impl PkZip {
    fn new(file_bytes: &[u8]) -> Self {
        let pkzip_magic_numbers : &[u8] = &[0x50, 0x4b, 0x03, 0x04];
        let pkzip_central_file_header_signature : &[u8] = &[0x50, 0x4b, 0x01, 0x02];
        let pkzip_end_of_central_directory_signature : &[u8] = &[0x50, 0x4b, 0x05, 0x06];

        let mut cursor = Cursor::new(file_bytes);
        // file_bytes.iter().position()
        //
        //

        //Find and seek to ECDR Signature
        let position_of_ecdr = file_bytes.windows(pkzip_end_of_central_directory_signature.len()).position(|x| x == pkzip_end_of_central_directory_signature).unwrap() as u64;
        println!("posiiton of ECDR sig {:#?}", position_of_ecdr);
        cursor.seek(std::io::SeekFrom::Start(position_of_ecdr)).unwrap();
        cursor.seek(std::io::SeekFrom::Current(4)).unwrap();
        let number_of_this_disk: &mut [u8;2] = &mut [0;2];
        let number_of_disk_with_start_of_central_directory: &mut [u8;2] = &mut [0;2];
        let total_number_of_entries_in_central_directory_on_current_disk: &mut [u8;2] = &mut [0;2];
        let total_number_of_entries_in_central_directory: &mut [u8;2] = &mut [0;2];
        let size_of_central_directory: &mut [u8;4] = &mut [0;4];
        let offset_of_start_of_central_directory_with_respect_to_starting_disk_number: &mut [u8;4] = &mut [0;4];
        let zip_file_comment_length: &mut [u8;2] = &mut [0;2];

        cursor.read_exact(number_of_this_disk).unwrap();
        cursor.read_exact(number_of_disk_with_start_of_central_directory).unwrap();
        cursor.read_exact(total_number_of_entries_in_central_directory_on_current_disk).unwrap();
        cursor.read_exact(total_number_of_entries_in_central_directory).unwrap();
        cursor.read_exact(size_of_central_directory).unwrap();
        cursor.read_exact(offset_of_start_of_central_directory_with_respect_to_starting_disk_number).unwrap();
        cursor.read_exact(zip_file_comment_length).unwrap();
        // Variable size
        // let zip_file_comment = Vec::with_capacity(u16::from_be_bytes(zip_file_comment_length.try_into().unwrap()) as usize);
        let zip_file_comment_length_value = u16::from_be_bytes(*zip_file_comment_length) as usize;
        let zip_file_comment = &mut Vec::with_capacity(zip_file_comment_length_value);
        cursor.read_exact(zip_file_comment).unwrap();

        let ECDR = EndOfCentralDirectoryRecord{

            number_of_this_disk: *number_of_this_disk,
            number_of_disk_with_start_of_central_directory: *number_of_disk_with_start_of_central_directory,
            total_number_of_entries_in_central_directory_on_current_disk: *total_number_of_entries_in_central_directory_on_current_disk,
            total_number_of_entries_in_central_directory: *total_number_of_entries_in_central_directory,
            size_of_central_directory: *size_of_central_directory,
            offset_of_start_of_central_directory_with_respect_to_starting_disk_number: *offset_of_start_of_central_directory_with_respect_to_starting_disk_number,
            zip_file_comment_length:*zip_file_comment_length,
            // Variable size
            zip_file_comment: zip_file_comment.to_vec()
        };
        println!("ECDR Parsed: {:#?}", ECDR);
        //Fill ECDR
        //Ensure there is only one file
        //Find and seek central directory records
        //Fill CDH
        //Go to file headers
        //Fill file headers
        //return struct

        todo!()
    }

}
#[derive(Debug)]
struct LocalFileHeader {

}
#[derive(Debug)]
struct CentralDirectoryHeader{
    version_maker :[u8;2],
    version_needed_to_extract: [u8;2],
    general_purpose_bit_flag : [u8;2],
    compression_method: [u8;2],
    last_mod_file_time: [u8;2],
    last_mod_file_date: [u8;2],
    crc_32: [u8;4],
    compressed_size: [u8;4],
    uncompressed_size: [u8;4],
    file_name_length: [u8;2],
    extra_field_length:[u8;2],
    file_comment_length:[u8;2],
    disk_number_start:[u8;2],
    internal_file_attributes:[u8;2],
    external_file_attributes:[u8;4],
    relative_offset_of_local_header:[u8;4],
    // Variable size
    file_name: Vec<u8>,
    extra_field: Vec<u8>,
    file_comment: Vec<u8>,
}
#[derive(Debug)]
struct EndOfCentralDirectoryRecord {
    number_of_this_disk: [u8;2],
    number_of_disk_with_start_of_central_directory: [u8;2],
    total_number_of_entries_in_central_directory_on_current_disk: [u8;2],
    total_number_of_entries_in_central_directory: [u8;2],
    size_of_central_directory: [u8;4],
    offset_of_start_of_central_directory_with_respect_to_starting_disk_number: [u8;4],
    zip_file_comment_length: [u8;2],
    // Variable size
    zip_file_comment: Vec<u8>
}


fn file_is_pkzip(file: &[u8]) -> bool
{
    let pkzip_magic_numbers : &[u8] = &[0x50, 0x4b, 0x03, 0x04];
    file.starts_with(pkzip_magic_numbers)
}

fn receive_new_dcc(read_loop_receiver: &mpsc::Receiver<IrcMessage>) -> IrcMessage {
    let dcc_send_request = loop {

        let dcc_send_request = read_loop_receiver.recv().unwrap();
        let sender = match dcc_send_request.prefix.as_ref().unwrap(){
            MessagePrefix::User { nickname, username: _, host: _ } => nickname,
            MessagePrefix::Server { servername } => servername,
        };
        println!("DCC SEND Request from {}. (y) to accept", sender);
        let mut buf = String::new();
        stdin().read_line(&mut buf).unwrap();
        if buf.starts_with('y')
        {
            break dcc_send_request;
        }
    };
    dcc_send_request
}

#[derive(Debug)]
pub struct DccConnection
{
    pub sock: TcpStream,
    pub reader: BufReader<TcpStream>,
}

impl DccConnection
{
    pub fn connect_raw(ip_address: &str) -> Result<DccConnection, &'static str>
    {
        println!("Attempting to connect to: {}", ip_address);
        let sock = TcpStream::connect(ip_address).unwrap();
        let reader = BufReader::new(sock.try_clone().unwrap());

        Ok(DccConnection { sock, reader })
    }
    pub fn connect(msg: IrcMessage) -> Result<DccConnection, &'static str>
    {
        match msg.command{
            MessageCommand::PRIVMSGCTCP { message_target: _, text: _, inner_message, inner_text: _, inner_params: _ } => match inner_message {
                Some(x) => match x{
                    CtcpMessage::DCC { query_type, argument: _, address, port } => match query_type{
                        DCCQueryType::SEND => {
                            
                            // let full_address = x.get_full_address().unwrap();
                            let full_address = CtcpMessage::get_full_address_from_strings(address,port).unwrap();
                            return DccConnection::connect_raw(full_address.as_str());
                        },
                        _ => Err("CTCP message was found, and it was a DCC request, but it was not a DCC Send")
                    },
                    _ => Err("CTCP message was found, but was not a DCC request")
                },
                None => Err("IrcMessage was a CTCP message, but it did not contain an innermessage"),
            },
            _ => Err("Non CTCP message")
        }
    }
    pub fn get_all_bytes(&mut self) -> Vec<u8>
    {
        let mut buf: Vec<u8> = Vec::new();
        self.reader.read_to_end(&mut buf).unwrap();
        buf
    }
}

fn read_loop(mut connex: IrcConnection, tx: std::sync::mpsc::Sender<IrcMessage>) -> !
{
    let mut buf = String::new();
    let mut reader = connex.get_reader();
    loop
    {
        reader.read_line(&mut buf).unwrap();
        buf = buf.trim_end_matches("\r\n").to_string();
        // println!("{:?}", buf);
        let message = IrcMessage::parse_message(&buf).unwrap();
        println!("{:?}", buf);
        println!("{:#?}", message);
        match message.command
        {
            MessageCommand::PING { token } =>
            {
                connex.send_command_args("PONG", token.as_str()).unwrap();
            }
            MessageCommand::PRIVMSGCTCP {
                message_target: _,
                text: _,
                inner_message,
                inner_text: _,
                inner_params: _,
            } => if let CtcpMessage::DCC {
                                query_type,
                                argument,
                                address,
                                port,
                            } = inner_message.unwrap() {
                match query_type
            {
                DCCQueryType::SEND =>
                {
                    println!("New file: {} on {}:{}", argument, address, port);
                    tx.send(IrcMessage::parse_message(&buf).unwrap()).unwrap();
                }
                DCCQueryType::CHAT => println!("Attempted chat {}:{}", address, port),
                _ =>
                {}
            }
            },
            MessageCommand::NONHANDLED =>
            {}
            _ =>
            {}
        }
        buf.clear();
    }
}

