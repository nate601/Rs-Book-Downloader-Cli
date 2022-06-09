use crate::message_prefix::MessagePrefix;

#[derive(Debug)]
pub struct IrcMessage
{
    pub prefix: Option<MessagePrefix>,
    pub command: MessageCommand,
    pub command_string: String,
    pub params: Vec<String>,
}

impl IrcMessage
{
    pub fn parse_message(message: &String) -> Result<IrcMessage, &'static str>
    {
        let mut split_message: Vec<&str> = message.split(" ").collect();
        let mut prefix: Option<MessagePrefix> = None;
        let mut params: Vec<String> = Vec::new();
        //See if message starts with prefix
        if message.starts_with(":")
        {
            //if it does, then record and drop the prefix.
            // prefix = Some(split_message[0].to_string());
            prefix = MessagePrefix::create_from_string(split_message[0].to_string()).ok();

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
                    message_target: params.get(0).unwrap().to_string(),
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
                            query_type: match inner_text_split[1].to_lowercase().as_str()
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
}
#[derive(Debug)]
pub enum MessageCommand
{
    PING
    {
        token: String,
    },
    PRIVMSG
    {
        message_target: String,
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
pub enum CtcpMessage
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
        query_type: DCCQueryType,
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

impl CtcpMessage
{
    /// Returns `true` if the ctcp message is [`DCC`].
    ///
    /// [`DCC`]: CtcpMessage::DCC
    pub fn is_dcc(&self) -> bool
    {
        matches!(self, Self::DCC { .. })
    }
    pub fn get_full_address(&self) -> Result<String, &'static str>
    {
        if let CtcpMessage::DCC {
            query_type,
            argument,
            address,
            port,
        } = &self
        {
            Ok(CtcpMessage::get_full_address_from_strings(address.to_string(),port.to_string()).unwrap().to_string())
        }
        else
        {
            return Err("Unable to get full_address of non DCC CtcpMessage");
        }
    }
    pub fn get_full_address_from_strings(
        address: String,
        port: String,
    ) -> Result<String, &'static str>
    {
        //TODO: remove these lines before connecting to non-test or non-local devices

        let converted_ip = CtcpMessage::convert_ip(address.to_string()).unwrap();
        let mut ret_val = String::new();
        // ret_val.push_str(converted_ip.as_str());
        ret_val.push_str("192.168.1.30");
        ret_val.push_str(":");
        ret_val.push_str(port.as_str());
        return Ok(ret_val);
    }
    fn convert_ip(start: String) -> Result<String, &'static str>
    {
        let int_val = {
            let this = start.parse::<i32>();
            match this
            {
                Ok(t) => t,
                Err(_e) => return Err("Unexpected value in convert_ip"),
            }
        };
        let y = int_val.to_be_bytes();
        let mut ret_val = String::new();
        ret_val.push_str(y[0].to_string().as_str());
        ret_val.push_str(".");
        ret_val.push_str(y[1].to_string().as_str());
        ret_val.push_str(".");
        ret_val.push_str(y[2].to_string().as_str());
        ret_val.push_str(".");
        ret_val.push_str(y[3].to_string().as_str());
        Ok(ret_val.to_string())
    }
}

#[derive(Debug)]
pub enum DCCQueryType
{
    SEND,
    CHAT,
    UNHANDLED,
}
