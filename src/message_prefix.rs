
#[derive(Debug)]
pub enum MessagePrefix
{
    User
    {
        nickname: String,
        username: String,
        host: String,
    },
    Server
    {
        servername: String
    },
}

impl MessagePrefix
{
    pub fn create_from_string(msg: String) -> Result<MessagePrefix, &'static str>
    {
        if msg.contains("@") && msg.contains("!")
        {
            let mut buf = msg;
            buf = buf.replace(":", "");
            buf = buf.replace("!", ":");
            buf = buf.replace("@", ":");

            let char_array: Vec<&str> = buf.split(":").collect();
            let ret_val = MessagePrefix::User {
                nickname: {
                    let this = char_array.get(0);
                    match this
                    {
                        Some(val) => val,
                        None => return Err("nickname not found when trying to parse prefix"),
                    }
                }
                .to_string(),
                username: {
                    let this = char_array.get(1);
                    match this
                    {
                        Some(val) => val,
                        None => return Err("username not found when trying to parse prefix"),
                    }
                }
                .to_string(),
                host: {
                    let this = char_array.get(2);
                    match this
                    {
                        Some(val) => val,
                        None => return Err("host not found when trying to parse prefix"),
                    }
                }
                .to_string(),
            };
            return Ok(ret_val);
        }
        else
        {
            let mut buf = msg;
            buf = buf.replace(":", "");
            return Ok(MessagePrefix::Server { servername: buf });
        }
    }
}
