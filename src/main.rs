use irc_connection::*;
use irc_message::*;
use message_prefix::*;
use pkzip::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::{stdin, BufReader};
use std::net::TcpStream;
use std::ops::DerefMut;
use std::sync::{mpsc, Arc, Mutex};
use std::{thread, time};

mod irc_connection;
mod irc_message;
mod message_prefix;
mod pkzip;
mod pkzip_test;

fn main()
{
    //Connect to server
    let mut connex = IrcConnection::connect("66.207.167.12:6660").unwrap();

    let mut users: HashMap<String, Vec<String>> = HashMap::new();
    let user_arc = Arc::new(Mutex::new(users));

    let user_arc_clone = Arc::clone(&user_arc);
    let (tx, rx) = mpsc::channel();
    //start read loop thread
    // read_loop(&mut connex);
    let read_connex = connex.try_clone().unwrap();
    let _read_loop_handle = thread::spawn(move || {
        read_loop(read_connex, tx, user_arc_clone);
    });
    //login to #bookz channel
    connex.send_command_args("NICK", "rapere").unwrap();
    connex
        .send_command_multiple_args("USER", vec!["rapere", " ", "8", " ", "*", " :nathan"])
        .unwrap();
    println!("Connecting... Please wait...");
    thread::sleep(time::Duration::from_secs(10));
    connex.send_command_args("JOIN", "#ebooks").unwrap();

    assert!(matches!(connex.status, ConnectionStatus::Connected));
    //Ask user for desired book
    let mut name = String::new();
    println!("Book Title?");
    stdin().read_line(&mut name).expect("Unable to read line");
    name = name.lines().take(1).collect::<String>();
    println!("Searching for {}.  Please wait...", name);
    println!("{:#?}", user_arc.lock().unwrap());

    name = format!("{}{}", "@search ", &name);
    //Request search results from SearchBox
    connex
        .send_message("#bookz", &name)
        .expect("Unable to send message");
    //wait to receive DCC Send request for packlist
    let (dcc_send_request, _) = wait_until_new_dcc(&rx);
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
    // eprintln!("pkzip = {:?}", pkzip);
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
    // println!("{:?}", decompressed_string);

    //: parse the txt file from Searchbot

    // eprintln!("split_string = {:#?}", split_string);

    let packlist = decompressed_string
        .split("\r\n")
        .filter(|x| x.starts_with('!'))
        .map(Pack::new)
        .filter(|x| {
            user_arc
                .lock()
                .unwrap()
                .get(&"bookz".to_string())
                .expect("No active bots for this book!")
                .contains(&x.bot_source)
        })
        .collect::<Vec<Pack>>();

    //:Present in a table the choices to the user

    //:Once user has selected choice, verify bot is online send a new message in the IRC channel
    let pack = loop
    {
        // print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
        println!("There are {} results.", { packlist.len() });

        for (i, e) in packlist.iter().take(5).enumerate()
        {
            println!(
                "{}:\tTitle: {}\n\tBot: {}\n\tAuthor: {}\n",
                i, e.book_title, e.bot_source, e.author
            );
        }
        let mut user_response = String::new();
        stdin().read_line(&mut user_response).unwrap();
        let number_resp_opt = user_response
            .lines()
            .take(1)
            .collect::<String>()
            .parse::<usize>();
        println!("{} {:#?}", user_response, number_resp_opt);
        if number_resp_opt.is_err()
        {
            continue;
        }
        let number_resp = number_resp_opt.unwrap();
        let pack_opt = packlist.get(number_resp);
        if (pack_opt.is_none())
        {
            continue;
        }
        let pack = pack_opt.unwrap();
        connex.send_message("#bookz", &pack.value).unwrap();
        break pack;
    };

    //:wait for DCC request then save file
    let (dcc_send_request, title) = wait_until_new_dcc(&rx);
    let mut dcc_connex = DccConnection::connect(dcc_send_request).unwrap();
    let file_data = dcc_connex.get_all_bytes();
    // println!("{:#?}", file_data);
    let mut file = File::create(title).unwrap();
    file.write_all(&file_data).unwrap();
    println!("Thank you, come again!");
}

#[derive(Debug, Clone)]
struct Pack
{
    value: String,
    bot_source: String,
    author: String,
    book_title: String,
}

impl Pack
{
    pub fn new(value: &str) -> Self
    {
        let mut edited_value = value.to_string();
        let bot_source = value.split(' ').collect::<Vec<&str>>()[0]
            .chars()
            .skip(1)
            .collect::<String>();
        edited_value = edited_value
            .chars()
            .skip(1 + bot_source.chars().count() + 1)
            .collect::<String>();
        let author = edited_value.split('-').take(1).collect::<String>();
        edited_value = edited_value
            .chars()
            .skip(author.chars().count() + 2)
            .collect::<String>();
        // println!("{}", edited_value);
        let mut was_last_character_info_marker = false;
        let junk_chars = [',', '_', '.', '-', ':'];
        let book_title = edited_value
            .chars()
            .take_while(|x| {
                if (*x == ':')
                {
                    if (was_last_character_info_marker)
                    {
                        false
                    }
                    else
                    {
                        was_last_character_info_marker = true;
                        true
                    }
                }
                else
                {
                    was_last_character_info_marker = false;
                    true
                }
            })
            .filter(|x| !junk_chars.contains(x))
            .collect::<String>();

        Self {
            value: value.to_string(),
            bot_source,
            author,
            book_title,
        }
    }
}

fn wait_until_new_dcc(
    read_loop_receiver: &mpsc::Receiver<(IrcMessage, String)>,
) -> (IrcMessage, String)
{
    let (dcc_send_request, title) = loop
    {
        let (dcc_send_request, title) = read_loop_receiver.recv().unwrap();
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
            break (dcc_send_request, title);
        }
    };
    (dcc_send_request, title.to_string())
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

fn read_loop(
    mut connex: IrcConnection,
    tx: std::sync::mpsc::Sender<(IrcMessage, String)>,
    users: Arc<Mutex<HashMap<String, Vec<String>>>>,
) -> !
{
    let mut buf = String::new();
    let mut reader = connex.get_reader();
    loop
    {
        {
            let this = reader.read_line(&mut buf);
            match this {
                Ok(t) => t,
                Err(_) => {continue;}
            }
        };
        buf = buf.trim_end_matches("\r\n").to_string();
        // println!("{:?}", buf);
        let message = IrcMessage::parse_message(&buf).unwrap();
        // println!("{:?}", buf);
        // println!("{:#?}", message);
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
                            tx.send((
                                IrcMessage::parse_message(&buf).unwrap(),
                                argument.to_string(),
                            ))
                            .unwrap();
                        }
                        DCCQueryType::CHAT => println!("Attempted chat {}:{}", address, port),
                        _ =>
                        {}
                    }
                }
            }
            MessageCommand::RPL_NAME_REPLY { channel, names } =>
            {
                let mut users = users.lock().unwrap();
                let cur_vec = users.get(&channel);
                if cur_vec.is_none()
                {
                    users.insert(channel, names);
                }
                else
                {
                    let mut new_vec = cur_vec
                        .unwrap()
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>();
                    new_vec.extend(names);
                    users.insert(channel, new_vec);
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
