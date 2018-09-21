use std::collections::HashMap;

#[derive(Debug)]
pub struct BotRequest {
    req_type: BotRequestType,

    sender_id: String,
    group_id: String,
    is_in_group: bool,

    cmd_arg: String,
    image: String,
}

#[derive(Debug)]
enum BotRequestType {
    Ignore,

    Help,
    HelpAdmin,
    About,
    PushImg,
    Set,
    Query,
    Random,

    Delete,
    Replace,
    Clean,
}

struct BotSession {
    prev_image: String,
}

pub struct BotGlobals {
    admin_id: String,
    sessions: HashMap<String, BotSession>,
}

impl BotGlobals {
    pub fn new(admin_id: String) -> BotGlobals {
        return BotGlobals { admin_id, sessions: HashMap::new() };
    }
}

impl BotRequest {
    pub fn new(params: &HashMap<String, String>, globals: &BotGlobals) -> BotRequest {
        let mut message = get(&params, "Message");
        let sender_id = get(&params, "QQ");
        let group_id = get(&params, "ExternalId");
        let bot_self_id = get(&params, "RobotQQ");
        let bot_self_name = get(&params, "Name");

        // ignore self message
        if bot_self_id == sender_id {
            return BotRequest::empty();
        }

        // if in group, handle @tutu messages only
        let is_in_group = !group_id.is_empty();
        let mut is_someone_at_tutu = false;
        if is_in_group {
            let bot_self_id = format!("[@{}] ", bot_self_id);
            let bot_self_name = format!("@{} ", bot_self_name);

            // someone @tutu
            if message.contains(bot_self_id.as_str()) || message.contains(bot_self_name.as_str()) {
                is_someone_at_tutu = true;

                // remote [@tutu] str
                message = message.replace(bot_self_id.as_str(), "");
                message = message.replace(bot_self_name.as_str(), "");
            }
        }

        // parse image
        let is_contain_images = false;
        // TODO
        let image = String::new();

        // parse command
        let message = message.trim().to_lowercase();
        let message = message.as_ref();
        let (req_type, cmd_arg) = if is_in_group {
            if is_someone_at_tutu {
                match message {
                    "help" => (BotRequestType::Help, String::new()),
                    "about" => (BotRequestType::About, String::new()),
                    "set" => (BotRequestType::Set, String::new()),
                    "random" => (BotRequestType::Random, String::new()),
                    _ => (BotRequestType::Query, String::new()),
                }
            } else {
                (BotRequestType::PushImg, String::new())
            }
        } else {
            if sender_id == globals.admin_id {
                match message {
                    "help" => (BotRequestType::HelpAdmin, String::new()),
                    "about" => (BotRequestType::About, String::new()),
                    "set" => (BotRequestType::Set, String::new()),
                    "random" => (BotRequestType::Random, String::new()),
                    "delete" => (BotRequestType::Delete, String::new()),
                    "replace" => (BotRequestType::Replace, String::new()),
                    "clean" => (BotRequestType::Clean, String::new()),
                    _ => if is_contain_images { (BotRequestType::PushImg, String::new()) } else { (BotRequestType::Query, String::new()) },
                }
            } else {
                return BotRequest::empty();
            }
        };

        println!("{}, {:?}", message, req_type);

        return BotRequest {
            req_type,

            sender_id,
            group_id,
            is_in_group,

            cmd_arg,
            image,
        };

        fn get(map: &HashMap<String, String>, k: &str) -> String {
            return if map.contains_key(k) { map[k].clone() } else { String::new() };
        }
    }

    fn empty() -> BotRequest {
        return BotRequest {
            req_type: BotRequestType::Ignore,

            sender_id: String::new(),
            group_id: String::new(),
            is_in_group: false,

            cmd_arg: String::new(),
            image: String::new(),
        };
    }
}

#[derive(Debug)]
pub struct BotResponse {
    pub resp_type: BotResponseType,
    pub target_id: String,
    pub text: String,
}

#[derive(Debug)]
pub enum BotResponseType {
    SendMessage,
    SendClusterMessage,
}

pub fn process_request(req: &BotRequest, globals: &mut BotGlobals) -> Vec<BotResponse> {
    let session_key = if req.is_in_group {
        format!("g{}", req.group_id)
    } else {
        format!("{}", req.sender_id)
    };

    if !globals.sessions.contains_key(session_key.as_str()) {
        globals.sessions.insert(session_key.clone(), BotSession { prev_image: String::new() });
    }

    let mut session = globals.sessions.get_mut(&session_key).unwrap();

    let req_type = &req.req_type;
    println!("{:?}", req);
    return match req_type {
        BotRequestType::Ignore => vec!(),
        BotRequestType::Help => simple(handle_help(), &req),
        BotRequestType::HelpAdmin => simple(handle_help_admin(), &req),
        BotRequestType::About => simple(handle_about(), &req),
        BotRequestType::PushImg => handle_push_img(&req, &mut session),
        BotRequestType::Set => simple(handle_set(&req, &session), &req),
        BotRequestType::Query => handle_query(&req),
        BotRequestType::Random => simple(handle_random(&req), &req),
        BotRequestType::Delete => simple(handle_delete(&req, &session), &req),
        BotRequestType::Replace => simple(handle_replace(&req, &session), &req),
        BotRequestType::Clean => simple(handle_clean(&req), &req),
    };
}

fn simple(msg: String, req: &BotRequest) -> Vec<BotResponse> {
    if req.is_in_group {
        return vec!(BotResponse {
            resp_type: BotResponseType::SendClusterMessage,
            target_id: req.group_id.clone(),
            text: msg,
        });
    } else {
        return vec!(BotResponse {
            resp_type: BotResponseType::SendMessage,
            target_id: req.sender_id.clone(),
            text: msg,
        });
    }
}

fn handle_help() -> String {
    return String::from("
=============
tutu bot v3.0
=============
* help
  显示本说明
* 直接发文字
  查询包含指定文字的图片
* set 字符串
  设置前一张图片对应的文字
* set [图片] 字符串 或 set 字符串 [图片]
  设置指定图片对应的文字
* random
  随机输出一张图片
* about
  显示版本说明
 "
    );
}

fn handle_help_admin() -> String {
    return String::from("
=======================
tutu bot admin commands
=======================
* help
  显示本说明
* 直接发文字
  查询包含指定文字的图片
* set 字符串
  设置前一张图片对应的文字
* set [图片] 字符串 或 set 字符串 [图片]
  设置指定图片对应的文字
* random
  随机输出一张图片
* about
  显示版本说明

* replace
  替换一张图片对应的文字（set命令是追加）
* delete
  从数据库中删除前一张图片的信息
* delete [图片]
  从数据库中删除指定的图片信息
 "
    );
}

fn handle_about() -> String {
    return String::from("
===========
about tutu
==========
2018-09-16 v3.0 rust
2017-08-13 v2.0 spring-boot
2017-05-21 v1.0 python

github: https://github.com/nuclearg/tutuv3

 "
    );
}

fn handle_push_img(req: &BotRequest, session: &mut BotSession) -> Vec<BotResponse> {
    if req.image.len() > 0 {
        session.prev_image = req.image.clone();
    }
    return vec!();
}

fn handle_set(req: &BotRequest, session: &BotSession) -> String {
    return String::from("");
}

fn handle_query(req: &BotRequest) -> Vec<BotResponse> {
    return vec!();
}

fn handle_random(req: &BotRequest) -> String {
    return String::from("");
}

fn handle_delete(req: &BotRequest, session: &BotSession) -> String {
    return String::from("");
}

fn handle_replace(req: &BotRequest, session: &BotSession) -> String {
    return String::from("");
}

fn handle_clean(req: &BotRequest) -> String {
    return String::from("");
}

