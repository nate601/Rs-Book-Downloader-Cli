use std::io::prelude::*;
use std::io::stdin;
use std::io::BufReader;
use std::net::TcpStream;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

type DccConnection = TcpStream;

enum ConnectionStatus
{
    PreConnect,
    Connected,
    WaitingForResults,
    WaitingForBook,
}

fn main()
{
    //Connect to server
    let mut connex = IrcConnection::connect("127.0.0.1:6667").unwrap();

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
    let mut buf = String::new();
    loop
    {
        if connex.reader.fill_buf().unwrap().is_empty()
        {
            println!("No new data, waiting 1 sec");
            sleep(Duration::from_secs(1));
        }
        else
        {
            connex.reader.read_line(&mut buf).unwrap();
            println!("{:?}", buf);
            buf.clear();
        }
            
    }
}

struct IrcConnection
{
    sock: TcpStream,
    status: ConnectionStatus,
    reader: BufReader<TcpStream>,
}
impl IrcConnection
{
    fn connect(ip_address: &str) -> Result<IrcConnection, &'static str>
    {
        let sock = TcpStream::connect(ip_address);
        match sock
        {
            Ok(v) => Ok(IrcConnection {
                reader: BufReader::new(v.try_clone().unwrap()),
                sock: v,
                status: ConnectionStatus::Connected,
            }),
            Err(_e) => panic!("Unable to connect to server."),
        }
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
