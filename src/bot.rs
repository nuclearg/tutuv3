use db;
use db::DbInfo;
use std::collections::HashMap;

const PIC_START: &str = "[图片=";
const PIC_END: &str = "/]";
const PIC_START_LEN: usize = 8;
const PIC_END_LEN: usize = 2;

#[derive(Debug)]
pub struct BotRequest {
    req_type: BotRequestType,

    sender_id: String,
    group_id: String,
    is_in_group: bool,

    word: String,
    pic: String,

    db: DbInfo,
}

#[derive(Debug)]
enum BotRequestType {
    Ignore,

    Help,
    HelpAdmin,
    About,
    RecordPrevImg,
    Set,
    Query,
    Random,

    Delete,
    Replace,
    Info,
    Count,
    Clean,
}

#[derive(Debug)]
struct BotSession {
    prev_pic: String,
}

#[derive(Debug)]
pub struct BotGlobals {
    admin_id: String,
    sessions: HashMap<String, BotSession>,
    db_user: String,
    db_pwd: String,
}

impl BotGlobals {
    pub fn new(admin_id: String, db_user: String, db_pwd: String) -> BotGlobals {
        return BotGlobals { admin_id, sessions: HashMap::new(), db_user, db_pwd };
    }
}

impl BotRequest {
    pub fn new(params: &HashMap<String, String>, globals: &BotGlobals) -> BotRequest {
        let message = get(&params, "Message");
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
        let message = match is_in_group {
            true => {
                let bot_self_id = format!("[@{}] ", bot_self_id);
                let bot_self_name = format!("@{} ", bot_self_name);

                // someone @tutu
                let mut message = message;
                if message.contains(&bot_self_id) || message.contains(&bot_self_name) {
                    is_someone_at_tutu = true;

                    // remote [@tutu] str
                    message = message.replace(&bot_self_id, "");
                    message = message.replace(&bot_self_name, "");
                }

                message
            }
            false => message
        };

        // parse image
        let (pic, text) = parse_pics(&message);
        let (cmd, mut word) = parse_cmd(&text);

        // parse command
        let req_type = if is_in_group {
            if is_someone_at_tutu {
                match cmd.as_str() {
                    "help" => BotRequestType::Help,
                    "about" => BotRequestType::About,
                    "set" => BotRequestType::Set,
                    "random" => BotRequestType::Random,
                    _ => if pic.is_empty() {
                        word = cmd;
                        BotRequestType::Query
                    } else { BotRequestType::RecordPrevImg }
                }
            } else {
                BotRequestType::RecordPrevImg
            }
        } else {
            if sender_id == globals.admin_id {
                match cmd.as_str() {
                    "help" => BotRequestType::HelpAdmin,
                    "about" => BotRequestType::About,
                    "set" => BotRequestType::Set,
                    "random" => BotRequestType::Random,
                    "delete" => BotRequestType::Delete,
                    "replace" => BotRequestType::Replace,
                    "info" => BotRequestType::Info,
                    "count" => BotRequestType::Count,
                    "clean" => BotRequestType::Clean,
                    _ => if pic.is_empty() {
                        word = cmd;
                        BotRequestType::Query
                    } else { BotRequestType::RecordPrevImg }
                }
            } else {
                return BotRequest::empty();
            }
        };

        return BotRequest {
            req_type,

            sender_id,
            group_id,
            is_in_group,

            word,
            pic,

            db: DbInfo::new(&globals.db_user, &globals.db_pwd),
        };

        fn get(map: &HashMap<String, String>, k: &str) -> String {
            return if map.contains_key(k) { map[k].clone() } else { String::new() };
        }

        fn parse_pics(message: &str) -> (String, String) {
            let mut image = String::new();

            let mut text = String::from(message);
            while text.contains(PIC_START) && text.contains(PIC_END) {
                let pos_start = text.find(PIC_START).unwrap();
                let pos_end = text.find(PIC_END).unwrap();

                if pos_end < pos_start {
                    return (image, String::new());
                }

                image = String::from(&text[pos_start + PIC_START_LEN..pos_end]);
                text = format!("{}{}", &text[0..pos_start], &text[pos_end + PIC_END_LEN..]);
            }

            return (image, text);
        }
        fn parse_cmd(text: &str) -> (String, String) {
            let text = text.trim();
            let pos = text.find(" ");
            if pos.is_none() {
                return (String::from(text), String::new());
            }
            return (
                String::from(&text[0..pos.unwrap()]),
                String::from(text[pos.unwrap() + 1..].trim()));
        }
    }

    fn empty() -> BotRequest {
        return BotRequest {
            req_type: BotRequestType::Ignore,

            sender_id: String::new(),
            group_id: String::new(),
            is_in_group: false,

            word: String::new(),
            pic: String::new(),

            db: DbInfo::empty(),
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

impl BotResponse {
    fn new(text: String, req: &BotRequest) -> BotResponse {
        if req.is_in_group {
            return BotResponse {
                resp_type: BotResponseType::SendClusterMessage,
                target_id: req.group_id.clone(),
                text,
            };
        } else {
            return BotResponse {
                resp_type: BotResponseType::SendMessage,
                target_id: req.sender_id.clone(),
                text,
            };
        }
    }

    fn simple(text: String, req: &BotRequest) -> Vec<BotResponse> {
        return vec!(BotResponse::new(text, req));
    }
}

pub fn process_request(req: &mut BotRequest, globals: &mut BotGlobals) -> Vec<BotResponse> {
    let session_key = if req.is_in_group {
        format!("g{}", req.group_id)
    } else {
        format!("{}", req.sender_id)
    };

    if !globals.sessions.contains_key(session_key.as_str()) {
        globals.sessions.insert(session_key.clone(), BotSession { prev_pic: String::new() });
    }
    let mut session = globals.sessions.get_mut(&session_key).unwrap();

    if req.pic.is_empty() {
        req.pic = session.prev_pic.clone();
    }

    let req_type = &req.req_type;
    return match req_type {
        BotRequestType::Ignore => vec!(),
        BotRequestType::Help => BotResponse::simple(handle_help(), &req),
        BotRequestType::HelpAdmin => BotResponse::simple(handle_help_admin(), &req),
        BotRequestType::About => BotResponse::simple(handle_about(), &req),
        BotRequestType::RecordPrevImg => handle_record_prev_img(&req, &mut session),
        BotRequestType::Set => BotResponse::simple(handle_set(&req), &req),
        BotRequestType::Query => handle_query(&req),
        BotRequestType::Random => BotResponse::simple(handle_random(&req), &req),
        BotRequestType::Delete => BotResponse::simple(handle_delete(&req), &req),
        BotRequestType::Replace => BotResponse::simple(handle_replace(&req), &req),
        BotRequestType::Info => BotResponse::simple(handle_info(&req), &req),
        BotRequestType::Count => BotResponse::simple(handle_count(&req), &req),
        BotRequestType::Clean => BotResponse::simple(handle_clean(&req), &req),
    };
}

fn handle_help() -> String {
    return String::from("tutu bot v3.0
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
    return String::from("tutu bot admin commands
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
* delete [图片]
  从数据库中删除指定的图片信息
* info [图片]
  查询一张图片下挂的所有词
* count
  查询现存的图片总数
* clean
  删除掉没有被引用的图片文件
 "
    );
}

fn handle_about() -> String {
    return String::from("about tutu
==========
2018-09-23 v3.0 rust
2017-08-13 v2.0 spring-boot
2017-05-21 v1.0 python
 "
    );
}

fn handle_record_prev_img(req: &BotRequest, session: &mut BotSession) -> Vec<BotResponse> {
    if !req.pic.is_empty() {
        session.prev_pic = req.pic.clone();
    }
    return vec!();
}

fn handle_set(req: &BotRequest) -> String {
    if req.pic.is_empty() {
        return String::from("set fail: no pic");
    }
    if req.word.is_empty() {
        return String::from("set fail: no text");
    }

    let result = db::append_word(&req.pic, &req.word, &req.db);
    return match result {
        Ok(_) => String::from("set ok"),
        Err(t) => format!("set fail: {}", t)
    };
}

fn handle_query(req: &BotRequest) -> Vec<BotResponse> {
    if req.word.is_empty() {
        return BotResponse::simple(String::from("query fail: no text"), req);
    }

    let result = db::query_pic(&req.word, &req.db);
    return match result {
        Ok(t) => if t.is_empty() {
            BotResponse::simple(String::from("query fail: not found"), req)
        } else {
            t.iter()
                .map(|pic| build_pic_output(pic))
                .map(|text| BotResponse::new(text, req))
                .collect()
        }
        Err(t) => BotResponse::simple(format!("query fail: {}", t), req)
    };
}

fn handle_random(req: &BotRequest) -> String {
    let result = db::random_pic(&req.db);
    return match result {
        Ok(t) => if t.is_empty() {
            format!("random fail: db empty")
        } else {
            build_pic_output(&t)
        },
        Err(t) => format!("random fail: {}", t)
    };
}

fn handle_delete(req: &BotRequest) -> String {
    if req.pic.is_empty() {
        return String::from("delete fail: no pic");
    }

    let result = db::delete_pic(&req.pic, &req.db);
    return match result {
        Ok(_) => String::from("delete ok"),
        Err(t) => format!("delete fail: {}", t)
    };
}

fn handle_replace(req: &BotRequest) -> String {
    if req.pic.is_empty() {
        return String::from("replace fail: no pic");
    }
    if req.word.is_empty() {
        return String::from("replace fail: no text");
    }

    let result = db::replace_word(&req.pic, &req.word, &req.db);
    return match result {
        Ok(_) => String::from("replace ok"),
        Err(t) => format!("replace fail: {}", t)
    };
}

fn handle_info(req: &BotRequest) -> String {
    if req.pic.is_empty() {
        return String::from("info fail: no pic");
    }

    let result = db::list_pic_words(&req.pic, &req.db);
    return match result {
        Ok(t) => format!("info ok: {}", t),
        Err(t) => format!("info fail: {}", t)
    };
}

fn handle_count(req: &BotRequest) -> String {
    let result = db::count_pic(&req.db);
    return match result {
        Ok(t) => format!("count ok: pic={}", t),
        Err(t) => format!("count fail: {}", t)
    };
}

fn handle_clean(req: &BotRequest) -> String {
    let result = db::clean(&req.db);
    return match result {
        Ok(t) => format!("clean ok: {}", t),
        Err(t) => format!("clean fail: {}", t)
    };
}

fn build_pic_output(pic: &str) -> String {
    return format!("{}{}{}", PIC_START, pic, PIC_END);
}
