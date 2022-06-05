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
pub enum DCCQueryType
{
    SEND,
    CHAT,
    UNHANDLED,
}
