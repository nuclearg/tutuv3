use mysql::{Error, Pool, QueryResult};
use mysql::prelude::FromValue;

#[derive(Debug)]
pub struct DbInfo {
    user: String,
    pwd: String,
}

impl DbInfo {
    pub fn new(user: &str, pwd: &str) -> DbInfo {
        return DbInfo { user: String::from(user), pwd: String::from(pwd) };
    }
    pub fn empty() -> DbInfo {
        return DbInfo { user: String::new(), pwd: String::new() };
    }
    fn conn(&self) -> Pool {
        let conn_string = format!("mysql://{}:{}@localhost:3306/tutu", self.user, self.pwd);
        return Pool::new(conn_string).unwrap();
    }
}

pub fn init(user: &str, pwd: &str) {
    DbInfo::new(user, pwd).conn();
}

pub fn append_word(pic: &str, word: &str, db: &DbInfo) -> Result<(), Error> {
    let conn = db.conn();

    let pic_id = find_pic_id_by_pic(pic, &conn)?;
    let pic_id = match pic_id {
        Some(t) => t,
        None => conn.prep_exec(
            "INSERT INTO t_pic (name) VALUES (:name)",
            params!("name" => pic))?.last_insert_id()
    };

    let words = word.split_whitespace();
    for word in words {
        let word_id = find_word_id_by_word(word, &conn)?;
        let word_id = match word_id {
            Some(t) => t,
            None => conn.prep_exec(
                "INSERT INTO t_word (word) VALUES (:word)",
                params!("word" => word))?.last_insert_id()
        };

        let assoc_id: Option<u64> = select_one(conn.prep_exec(
            "SELECT id FROM t_pic_word WHERE id_pic = :pic_id AND id_word = :word_id",
            params!("pic_id" => pic_id, "word_id" => word_id))?)?;
        if assoc_id.is_none() {
            conn.prep_exec(
                "INSERT INTO t_pic_word (id_pic, id_word) VALUES (:pic_id, :word_id)",
                params!("pic_id" => pic_id, "word_id" => word_id))?;
        }
    }

    return Ok(());
}

pub fn replace_word(pic: &str, word: &str, db: &DbInfo) -> Result<(), Error> {
    delete_pic(pic, db)?;
    append_word(pic, word, db)?;
    Ok(())
}

pub fn delete_pic(pic: &str, db: &DbInfo) -> Result<(), Error> {
    let conn = db.conn();

    let pic_id = find_pic_id_by_pic(pic, &conn)?;
    return match pic_id {
        Some(t) => {
            conn.prep_exec(
                "DELETE FROM t_pic_word WHERE id_pic = :pic_id",
                params!("pic_id" => t))?;
            Ok(())
        }
        None => Ok(())
    };
}

pub fn query_pic(word: &str, db: &DbInfo) -> Result<Vec<String>, Error> {
    let conn = db.conn();

    // 选出最新的一张图片
    let pic: Option<String> = select_one(conn.prep_exec(
        "SELECT name
         FROM t_pic p
         JOIN t_pic_word j ON j.id_pic = p.id
         JOIN t_word w ON j.id_word = w.id
         WHERE w.word = :word
         ORDER BY j.last_ts
         LIMIT 1",
        params!("word" => &word))?)?;
    if pic.is_none() {
        return Ok(vec!());
    }
    let pic = pic.unwrap();

    // 以及随机的一张图片
    let pic2 = select_one(conn.prep_exec(
        "SELECT name
         FROM t_pic p
         JOIN t_pic_word j ON j.id_pic = p.id
         JOIN t_word w ON j.id_word = w.id
         WHERE w.word = :word
           AND p.name != :exclude
         ORDER BY rand()
         LIMIT 1",
        params!("word" => &word, "exclude" => &pic))?)?;
    return match pic2 {
        Some(t) => Ok(vec!(pic, t)),
        None => Ok(vec!(pic))
    };
}

pub fn random_pic(db: &DbInfo) -> Result<String, Error> {
    let conn = db.conn();

    // 选出最新的一张图片
    let pic: Option<String> = select_one(conn.prep_exec(
        "SELECT name
         FROM t_pic p
         ORDER BY rand()
         LIMIT 1",
        ())?)?;

    return match pic {
        Some(t) => Ok(t),
        None => Ok(String::new())
    };
}

pub fn clean(db: &DbInfo) -> Result<String, String> {
    return Err(String::from("// TODO clean()"));
}


fn find_pic_id_by_pic(pic: &str, conn: &Pool) -> Result<Option<u64>, Error> {
    return select_one(conn.prep_exec(
        "SELECT id FROM t_pic WHERE name = :name",
        params!("name" => pic))?);
}

fn find_word_id_by_word(word: &str, conn: &Pool) -> Result<Option<u64>, Error> {
    return select_one(conn.prep_exec(
        "SELECT id FROM t_word WHERE word = :word",
        params!("word" => word))?);
}

fn select_one<T>(result: QueryResult) -> Result<Option<T>, Error>
    where T: FromValue
{
    return match result.last() {
        Some(t) => {
            let row = t.unwrap();
            let id: Option<T> = row.get(0);
            Ok(id)
        }
        None => Ok(None)
    };
}

