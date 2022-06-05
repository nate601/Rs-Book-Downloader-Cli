use std::io::prelude::*;
use std::io::{stdin, BufReader};
use std::net::TcpStream;
use std::thread;

enum ConnectionStatus
{
    Disconnected,
    Connected,
    WaitingForResults,
    WaitingForBook,
}

fn main()
{
    //Connect to server
    let mut connex = IrcConnection::connect("127.0.0.1:6667").unwrap();

    //start read loop thread
    // read_loop(&mut connex);
    let read_connex = connex.try_clone().unwrap();
    let _read_loop_handle = thread::spawn(move || {
        read_loop(read_connex);
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
    //wait to receive PackList
}

fn read_loop(mut connex: IrcConnection) -> !
{
    let mut buf = String::new();
    let mut reader = connex.get_reader();
    loop
    {
        reader.read_line(&mut buf).unwrap();
        buf = buf.trim_end_matches("\r\n").to_string();
        // println!("{:?}", buf);
        let message = parse_message(&buf).unwrap();
        println!("{:?}", buf);
        println!("{:#?}", message);
        match message.command
        {
            MessageCommand::PING { token } =>
            {
                connex.send_command_args("PONG", token.as_str()).unwrap();
            }
            MessageCommand::PRIVMSGCTCP {
                message_target,
                text,
                inner_message,
                inner_text,
                inner_params,
            } =>
            {
                match inner_message.unwrap(){
                    CtcpMessage::DCC { queryType, argument, address, port } => {
                        match queryType{
                            DCCQueryType::SEND => println!("New file: {} on {}:{}", argument, address, port),
                            DCCQueryType::CHAT => println!("Attempted chat {}:{}", address, port),
                            _ => {}
                        }
                    },
                    _ => {}
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
#[derive(Debug)]
enum MessageCommand
{
    PING
    {
        token: String,
    },
    PRIVMSG
    {
        messageTarget: String,
        text: String,
    },
    PRIVMSGCTCP
    {
        message_target: String,
        text: String,
        inner_message: Option<CtcpMessage>,
        inner_text: String,
        inner_params: Vec<String>,
    },
    NONHANDLED,
    EMPTY,
}
#[derive(Debug)]
enum CtcpMessage
{
    ACTION
    {
        text: String,
    },
    CLIENTINFO
    {
        token: String,
    },
    DCC
    {
        queryType: DCCQueryType,
        argument: String,
        address: String,
        port: String,
    },
    PING
    {
        token: String,
    },
    UNHANDLED,
}
#[derive(Debug)]
enum DCCQueryType
{
    SEND,
    CHAT,
    UNHANDLED,
}
#[derive(Debug)]
struct IrcMessage
{
    prefix: Option<String>,
    command: MessageCommand,
    command_string: String,
    params: Vec<String>,
}

fn parse_message(message: &String) -> Result<IrcMessage, &'static str>
{
    let mut split_message: Vec<&str> = message.split(" ").collect();
    let mut prefix: Option<String> = None;
    let mut params: Vec<String> = Vec::new();
    //See if message starts with prefix
    if message.starts_with(":")
    {
        //if it does, then record and drop the prefix.
        prefix = Some(split_message[0].to_string());
        split_message.remove(0);
    }
    let command_string = split_message[0];
    split_message.remove(0);

    loop
    {
        if split_message.len() == 0 || split_message[0].starts_with(":")
        {
            break;
        }
        params.push(split_message[0].to_string());
        split_message.remove(0);
    }
    if split_message.len() != 0
    {
        let mut trailing = split_message.join(" ");
        trailing = trailing[1..trailing.len()].to_string();
        params.push(trailing);
    }

    let command = match command_string.to_lowercase().as_str()
    {
        "ping" => MessageCommand::PING {
            token: params.get(0).unwrap().to_string(),
        },
        "privmsg" if params.get(1).unwrap().to_string().starts_with("\u{1}") == false =>
        {
            MessageCommand::PRIVMSG {
                messageTarget: params.get(0).unwrap().to_string(),
                text: params.get(1).unwrap().to_string(),
            }
        }
        "privmsg" if params.get(1).unwrap().to_string().starts_with("\u{1}") == true =>
        {
            let inner_text = params
                .get(1)
                .unwrap()
                .trim_start_matches("\u{1}")
                .trim_end_matches("\u{1}")
                .to_string();
            let inner_text_split: Vec<String> =
                inner_text.split(" ").map(|x| x.to_string()).collect();
            // println!("#### INNER_TEXT_SPLIT {:#?}",inner_text_split);
            MessageCommand::PRIVMSGCTCP {
                message_target: params.get(0).unwrap().to_string(),
                text: params.get(1).unwrap().to_string(),
                inner_message: match inner_text_split[0].to_lowercase().as_str()
                {
                    //#### INNER_TEXT_SPLIT [
                    //    "DCC",
                    //    "CHAT",
                    //    "chat",
                    //    "413319771",
                    //    "1023",
                    //]
                    "dcc" => Some(CtcpMessage::DCC {
                        queryType: match inner_text_split[1].to_lowercase().as_str()
                        {
                            "send" => DCCQueryType::SEND,
                            "chat" => DCCQueryType::CHAT,
                            _ => DCCQueryType::UNHANDLED,
                        },
                        argument: inner_text_split[2].to_string(),
                        address: inner_text_split[3].to_string(),
                        port: inner_text_split[4].to_string(),
                    }),
                    _ => Some(CtcpMessage::UNHANDLED),
                },
                inner_text,
                inner_params: inner_text_split,
            }
        }
        _ => MessageCommand::NONHANDLED,
    };
    return Ok(IrcMessage {
        prefix,
        command,
        command_string: command_string.to_string(),
        params,
    });
}

struct IrcConnection
{
    sock: TcpStream,
    status: ConnectionStatus,
}
impl IrcConnection
{
    fn connect(ip_address: &str) -> Result<IrcConnection, &'static str>
    {
        let sock = TcpStream::connect(ip_address);
        match sock
        {
            Ok(v) => Ok(IrcConnection {
                sock: v,
                status: ConnectionStatus::Connected,
            }),
            Err(_e) => panic!("Unable to connect to server."),
        }
    }
    fn get_reader(&self) -> BufReader<TcpStream>
    {
        BufReader::new(self.sock.try_clone().unwrap())
    }
    fn new_from_stream(sock: &TcpStream) -> Result<IrcConnection, &'static str>
    {
        Ok(IrcConnection {
            sock: sock.try_clone().unwrap(),
            status: ConnectionStatus::Connected,
        })
    }
    fn send_bytes(&mut self, bytes: &[u8]) -> Result<usize, &'static str>
    {
        let written_byte_size = self.sock.write(bytes);
        match written_byte_size
        {
            Ok(v) => return Ok(v),
            Err(_e) => return Err("Unable to send data on TCP socket"),
        }
    }
    fn send_command(&mut self, command: &str) -> Result<usize, &'static str>
    {
        self.send_string(command)
    }
    fn send_command_args(&mut self, command: &str, arguments: &str) -> Result<usize, &'static str>
    {
        let message = format!("{} {}\n", command, arguments);
        return self.send_string(&message);
    }
    fn send_command_multiple_args(
        &mut self,
        command: &str,
        arguments: Vec<&str>,
    ) -> Result<usize, &'static str>
    {
        return self.send_command_args(command, arguments.join("").as_str());
    }
    fn send_message(&mut self, channel: &str, message: &str) -> Result<usize, &'static str>
    {
        self.send_command_multiple_args("PRIVMSG", vec![channel, " :", message])
    }
    fn send_string(&mut self, message: &str) -> Result<usize, &'static str>
    {
        println!("Sending: {}", message);
        self.send_bytes(message.as_bytes())
    }
    fn try_clone(&self) -> std::io::Result<IrcConnection>
    {
        Ok(IrcConnection::new_from_stream(&self.sock.try_clone().unwrap()).unwrap())
    }
}
