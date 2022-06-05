use std::io::prelude::*;
use std::io::{stdin, BufReader};
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use message_prefix::*;
use irc_message::*;

mod message_prefix;
mod irc_message;

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
    let dcc_send_request = rx.recv().unwrap();
    println!("Send request received {:#?} ", dcc_send_request)
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
            } => match inner_message.unwrap()
            {
                CtcpMessage::DCC {
                    queryType,
                    argument,
                    address,
                    port,
                } => match queryType
                {
                    DCCQueryType::SEND =>
                    {
                        println!("New file: {} on {}:{}", argument, address, port);
                        tx.send(IrcMessage::parse_message(&buf).unwrap()).unwrap();
                    }
                    DCCQueryType::CHAT => println!("Attempted chat {}:{}", address, port),
                    _ =>
                    {}
                },
                _ =>
                {}
            },
            MessageCommand::NONHANDLED =>
            {}
            _ =>
            {}
        }
        buf.clear();
    }
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
