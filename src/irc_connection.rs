use std::{
    io::{BufReader, Write},
    net::TcpStream,
};

pub enum ConnectionStatus
{
    Disconnected,
    Connected,
    WaitingForResults,
    WaitingForBook,
}
pub struct IrcConnection
{
    pub sock: TcpStream,
    pub status: ConnectionStatus,
}
impl IrcConnection
{
    pub fn connect(ip_address: &str) -> Result<IrcConnection, &'static str>
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
    pub fn get_reader(&self) -> BufReader<TcpStream>
    {
        BufReader::new(self.sock.try_clone().unwrap())
    }
    pub fn new_from_stream(sock: &TcpStream) -> Result<IrcConnection, &'static str>
    {
        Ok(IrcConnection {
            sock: sock.try_clone().unwrap(),
            status: ConnectionStatus::Connected,
        })
    }
    pub fn send_bytes(&mut self, bytes: &[u8]) -> Result<usize, &'static str>
    {
        let written_byte_size = self.sock.write(bytes);
        match written_byte_size
        {
            Ok(v) => return Ok(v),
            Err(_e) => return Err("Unable to send data on TCP socket"),
        }
    }
    pub fn send_command(&mut self, command: &str) -> Result<usize, &'static str>
    {
        self.send_string(command)
    }
    pub fn send_command_args(&mut self, command: &str, arguments: &str) -> Result<usize, &'static str>
    {
        let message = format!("{} {}\n", command, arguments);
        return self.send_string(&message);
    }
    pub fn send_command_multiple_args(
        &mut self,
        command: &str,
        arguments: Vec<&str>,
    ) -> Result<usize, &'static str>
    {
        return self.send_command_args(command, arguments.join("").as_str());
    }
    pub fn send_message(&mut self, channel: &str, message: &str) -> Result<usize, &'static str>
    {
        self.send_command_multiple_args("PRIVMSG", vec![channel, " :", message])
    }
    pub fn send_string(&mut self, message: &str) -> Result<usize, &'static str>
    {
        println!("Sending: {}", message);
        self.send_bytes(message.as_bytes())
    }
    pub fn try_clone(&self) -> std::io::Result<IrcConnection>
    {
        Ok(IrcConnection::new_from_stream(&self.sock.try_clone().unwrap()).unwrap())
    }
}
