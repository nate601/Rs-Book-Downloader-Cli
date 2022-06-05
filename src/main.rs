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
    let mut reader = connex.get_reader();
    let readLoopHandle = thread::spawn(move || {

        read_loop(&mut reader);
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
}

fn read_loop(reader: &mut BufReader<TcpStream>) -> ! {
    let mut buf = String::new();
    loop
    {
        reader.read_line(&mut buf).unwrap();
        println!("{:?}", buf);
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
    fn send_message(&mut self, channel: &str, message: &str) -> Result<usize, &'static str>
    {
        self.send_command_multiple_args("PRIVMSG", vec![channel, " :", message])
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
    fn send_string(&mut self, message: &str) -> Result<usize, &'static str>
    {
        self.send_bytes(message.as_bytes())
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
}
