use std::io::prelude::*;
use std::net::TcpStream;
use std::thread;

type IrcConnection = TcpStream;
type DccConnection = TcpStream;

fn main() {
    // let ip = 413319771i32;
    // let bytes = ip.to_be_bytes();
    // for i in bytes {
    //     println!("{}", i);

    // }
    let sock = connect("127.0.0.1:6667").unwrap();

    send_command_args(&sock, "NICK", "rapere").unwrap();
    send_command_multiple_args(&sock, "USER", vec!("rapere", " ", "8", " ", "*", " :nathan")).unwrap();
    send_command_args(&sock, "JOIN", "#bookz").unwrap();
    send_message(&sock, "#bookz", "Hello World!").unwrap();
}
fn connect(ip_address: &str) -> Result<IrcConnection, &'static str> 
{
    let sock = IrcConnection::connect(ip_address);
    match sock {
        Ok(v) => Ok(v),
        Err(_e) => Err("Unable to connect to server."),
    }
}
    
fn send_message(sock: &IrcConnection, channel: &str, message: &str) -> Result<usize, &'static str> 
{
    send_command_multiple_args(&sock,"PRIVMSG", vec!(channel, " :", message))
}
fn send_command(sock: &IrcConnection, command: &str) -> Result<usize, &'static str>
{
    send_string(&sock, command)
}
fn send_command_args(sock: &IrcConnection, command: &str, arguments: &str) -> Result<usize, &'static str>
{
    let message = format!("{} {}\n", command, arguments);
    return send_string(sock, &message);
}

fn send_command_multiple_args(sock: &IrcConnection, command: &str, arguments: Vec<&str>) -> Result<usize, &'static str>
{
    send_command_args(sock, command, arguments.join("").as_str())
}
fn send_string(sock: &IrcConnection, message: &str) -> Result<usize, &'static str> 
{
    send_bytes(&sock, message.as_bytes())
}
fn send_bytes(mut sock: &IrcConnection, bytes: &[u8]) -> Result<usize, &'static str> 
{
    let written_byte_size = sock.write(bytes);
    match written_byte_size {
        Ok(v) => return Ok(v),
        Err(_e) => return Err("Unable to send data on TCP Socket"),
    }
}
