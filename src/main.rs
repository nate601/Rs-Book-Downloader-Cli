use irc_connection::*;
use irc_message::*;
use message_prefix::*;
use pkzip::*;
use std::io::prelude::*;
use std::io::{stdin, BufReader};
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;

mod irc_connection;
mod irc_message;
mod message_prefix;
mod pkzip;
mod pkzip_test;

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

    // println!("{:#?} ", zipped_results_file_bytes);

    //:verify file we received was pkzip file
    if PkZip::data_is_pkzip(&zipped_results_file_bytes)
    {
        println!("File is PKZIP!");
    }
    else
    {
        panic!(
            "Non PKZIP file sent when expected PKZIP {:#?}",
            zipped_results_file_bytes
        )
    }

    //:unzip file received thru DCC request
    let pkzip = PkZip::new(&zipped_results_file_bytes);
    eprintln!("pkzip = {:?}", pkzip);
    let pkzip_files = pkzip.get_files();
    // println!("pkzip files {:?}", pkzip_files);
    if pkzip_files.len() != 1
    {
        panic!("more than 1 file detected in zip file, should only have 1!");
    }
    let list_file = pkzip_files.first().unwrap();
    // println!("{:#?}", pkzip);
    // println!("{:#?}", pkzip_files);
    let decompressed_data = list_file.decompress().unwrap();
    // println!("{:?}", decompressed_data);
    let decompressed_string = String::from_utf8(decompressed_data).unwrap();
    println!("{:?}", decompressed_string);

    //: parse the txt file from Searchbot

    let split_string: Vec<&str> = decompressed_string
        .split("\r\n")
        .filter(|x| x.starts_with('!'))
        .collect();
    eprintln!("split_string = {:#?}", split_string);

    let packlist = split_string
        .into_iter()
        .map(|x| Pack::new(x))
        .collect::<Vec<Pack>>();

    //:Present in a table the choices to the user

    println!("There are {} results.", { packlist.len() });

    for (i, e) in packlist.into_iter().enumerate()
    {
        println!("{}: {}'s {}", i, e.author, e.book_title);
    }

    //:Once user has selected choice, verify bot is online send a new message in the IRC channel
    //

    //:wait for DCC request then save file
}

#[derive(Debug)]
struct Pack
{
    value: String,
    bot_source: String,
    author: String,
    book_title: String,
    file_size: String,
}

impl Pack
{
    pub fn new(value: &str) -> Self
    {
        let chunks_space = value.split(' ').collect::<Vec<&str>>();
        let bot_source = chunks_space[0].replace("!", "");
        let author_title_split = chunks_space[1].split('-').collect::<Vec<&str>>();
        let file_size = chunks_space.last().unwrap().to_string();
        let author = author_title_split[0].to_string();
        let book_title = author_title_split.last().unwrap().to_string();

        Self {
            value: value.to_string(),
            bot_source,
            file_size,
            author,
            book_title,
        }
    }
}

fn receive_new_dcc(read_loop_receiver: &mpsc::Receiver<IrcMessage>) -> IrcMessage
{
    let dcc_send_request = loop
    {
        let dcc_send_request = read_loop_receiver.recv().unwrap();
        let sender = match dcc_send_request.prefix.as_ref().unwrap()
        {
            MessagePrefix::User {
                nickname,
                username: _,
                host: _,
            } => nickname,
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
            } =>
            {
                if let CtcpMessage::DCC {
                    query_type,
                    argument,
                    address,
                    port,
                } = inner_message.unwrap()
                {
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
                }
            }
            MessageCommand::NONHANDLED =>
            {}
            _ =>
            {}
        }
        buf.clear();
    }
}
