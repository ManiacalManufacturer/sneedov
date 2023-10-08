use async_trait::async_trait;
use sqlite;

const INIT_QUERY: &str = "
PRAGMA synchronous = OFF;

CREATE TABLE IF NOT EXISTS Words(
    id INTEGER PRIMARY KEY,
    keyword VARCHAR(20),
    string VARCHAR(255),
    UNIQUE(keyword,string)
    );

CREATE TABLE IF NOT EXISTS Occurrence (
    prev INT NOT NULL,
    curr INT NOT NULL,
    next INT NOT NULL,
    occurrences INT,
    UNIQUE(prev, curr, next)
    );
    ";

const ADD_QUERY: &str = "
    INSERT OR IGNORE INTO Words (id, keyword, string) VALUES(
        null,
        :keyword,
        :string
        );
    ";

const INCREMENT_QUERY: &str = "
INSERT INTO Occurrence (prev, curr, next, occurrences) VALUES(:index1, :index2, :index3, 1)
    ON CONFLICT(prev, curr, next) DO UPDATE SET occurrences = occurrences + 1;
    ";

const GET_QUERY: &str = "
    SELECT string FROM Words WHERE id = :id;
    ";

const GET_CASE_INSENSITIVE: &str = "
    SELECT * FROM Words WHERE LOWER(string) = :string;
    ";

const SINGLE_NEXT_QUERY: &str = "
    SELECT * FROM Occurrence WHERE curr = :index;
    ";

const DOUBLE_NEXT_QUERY: &str = "
    SELECT * FROM Occurrence WHERE prev = :index1 AND curr = :index2;
    ";

const DOUBLE_PREV_QUERY: &str = "
    SELECT * FROM Occurrence WHERE curr = :index1 AND next = :index2;
    ";

const REVERSE_QUERY: &str = "
    SELECT occurrences FROM Occurrence WHERE curr = :index1 and next = :index2;
    ";

type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct SqliteDB {
    connection: sqlite::ConnectionWithFullMutex,
}

impl SqliteDB {
    pub async fn new(path: &std::path::Path) -> Result<Self, Error> {
        let db = SqliteDB {
            connection: sqlite::Connection::open_with_full_mutex(path)?,
        };
        db.connection.execute(INIT_QUERY)?;
        Ok(db)
    }
}

pub struct SqliteBlacklist {
    connection: sqlite::ConnectionWithFullMutex,
}

impl SqliteBlacklist {
    pub async fn new(path: &std::path::Path) -> Result<Self, Error> {
        let db = SqliteBlacklist {
            connection: sqlite::Connection::open_with_full_mutex(path)?,
        };
        db.connection.execute(
            "PRAGMA synchronous = OFF;
            
            CREATE TABLE IF NOT EXISTS Blacklist(
                chat_id INT NOT NULL,
                user_id INT NOT NULL,
                UNIQUE(chat_id, user_id)
                );
            ",
        )?;
        Ok(db)
    }
}

#[async_trait]
pub trait Database {
    async fn add_word(&self, tuple: (&str, &str)) -> Result<u64, Error>;

    async fn increment(&self, index1: u64, index2: u64, index3: u64) -> Result<(), Error>;

    async fn get_word(&self, index: u64) -> Result<String, Error>;

    async fn get_case_insensitive(&self, string: &str) -> Result<Vec<(u64, String)>, Error>;

    async fn get_single_occurrences(&self, index: u64) -> Result<Vec<(u64, u64)>, Error>;

    async fn get_double_occurrences(
        &self,
        index1: u64,
        index2: u64,
    ) -> Result<Vec<(u64, u64)>, Error>;

    async fn get_prev_single_occurrences(&self, index: u64) -> Result<Vec<(u64, u64)>, Error>;

    async fn get_prev_double_occurrences(
        &self,
        index1: u64,
        index2: u64,
    ) -> Result<Vec<(u64, u64)>, Error>;
}

#[async_trait]
pub trait Blacklist {
    async fn blacklist(&self, chat_id: i64, user_id: u64) -> Result<(), Error>;

    async fn unblacklist(&self, chat_id: i64, user_id: u64) -> Result<(), Error>;

    async fn is_blacklisted(&self, chat_id: i64, user_id: u64) -> Result<bool, Error>;
}

#[async_trait]
impl Database for SqliteDB {
    async fn add_word(&self, tuple: (&str, &str)) -> Result<u64, Error> {
        let mut statement = self.connection.prepare(ADD_QUERY)?;

        statement.bind_iter::<_, (_, sqlite::Value)>([
            (":keyword", tuple.0.clone().into()),
            (":string", tuple.1.clone().into()),
        ])?;

        while let Ok(sqlite::State::Row) = statement.next() {}

        let mut statement = self
            .connection
            .prepare("SELECT id FROM Words WHERE keyword = :keyword AND string = :string")?;
        statement.bind_iter::<_, (_, sqlite::Value)>([
            (":keyword", tuple.0.into()),
            (":string", tuple.1.into()),
        ])?;

        let mut id: i64 = 0;
        while let Ok(sqlite::State::Row) = statement.next() {
            id = statement.read::<i64, _>("id").unwrap();
        }

        return Ok(id as u64);
    }

    async fn increment(&self, index1: u64, index2: u64, index3: u64) -> Result<(), Error> {
        let mut statement = self.connection.prepare(INCREMENT_QUERY)?;
        statement.bind_iter::<_, (_, i64)>([
            (":index1", index1 as i64),
            (":index2", index2 as i64),
            (":index3", index3 as i64),
        ])?;
        while let Ok(sqlite::State::Row) = statement.next() {}
        Ok(())
    }

    async fn get_word(&self, index: u64) -> Result<String, Error> {
        let mut statement = self.connection.prepare(GET_QUERY)?;
        statement.bind((":id", index as i64))?;

        if let Ok(sqlite::State::Row) = statement.next() {
            Ok(statement.read::<String, _>("string")?)
        } else {
            let err: Error =
                String::from("None was returned. Is your file corrupted or missing?").into();
            Err(err)
        }
    }

    async fn get_case_insensitive(&self, string: &str) -> Result<Vec<(u64, String)>, Error> {
        let mut statement = self.connection.prepare(GET_CASE_INSENSITIVE)?;
        statement.bind((":string", string.to_lowercase().as_str()))?;

        let mut vec: Vec<(u64, String)> = vec![];
        while let Ok(sqlite::State::Row) = statement.next() {
            vec.push((
                statement.read::<i64, _>("id")? as u64,
                statement.read::<String, _>("keyword")?,
            ));
        }
        Ok(vec)
    }

    async fn get_single_occurrences(&self, index: u64) -> Result<Vec<(u64, u64)>, Error> {
        let mut statement = self.connection.prepare(SINGLE_NEXT_QUERY)?;
        statement.bind((":index", index as i64))?;

        let mut vec: Vec<(u64, u64)> = vec![];
        while let Ok(sqlite::State::Row) = statement.next() {
            vec.push((
                statement.read::<i64, _>("next")? as u64,
                statement.read::<i64, _>("occurrences")? as u64,
            ));
        }
        Ok(vec)
    }

    async fn get_double_occurrences(
        &self,
        index1: u64,
        index2: u64,
    ) -> Result<Vec<(u64, u64)>, Error> {
        let mut statement = self.connection.prepare(DOUBLE_NEXT_QUERY)?;
        statement
            .bind_iter::<_, (_, i64)>([(":index1", index1 as i64), (":index2", index2 as i64)])?;

        let mut vec: Vec<(u64, u64)> = vec![];
        while let Ok(sqlite::State::Row) = statement.next() {
            vec.push((
                statement.read::<i64, _>("next").unwrap() as u64,
                statement.read::<i64, _>("occurrences").unwrap() as u64,
            ));
        }
        Ok(vec)
    }

    async fn get_prev_single_occurrences(&self, index: u64) -> Result<Vec<(u64, u64)>, Error> {
        let mut statement = self.connection.prepare(SINGLE_NEXT_QUERY)?;
        statement.bind((":index", index as i64))?;

        let mut vec1: Vec<u64> = vec![];
        let mut vec2: Vec<u64> = vec![];
        while let Ok(sqlite::State::Row) = statement.next() {
            vec1.push(statement.read::<i64, _>("prev").unwrap() as u64);
        }

        if vec1.is_empty() {
            let err: Error = String::from("The length of vec is 0").into();
            return Err(err);
        }
        let prev = vec1[0];

        let mut statement = self.connection.prepare(REVERSE_QUERY)?;
        statement
            .bind_iter::<_, (_, i64)>([(":index1", prev as i64), (":index2", index as i64)])?;
        while let Ok(sqlite::State::Row) = statement.next() {
            vec2.push(statement.read::<i64, _>("occurrences").unwrap() as u64);
        }

        let vec = vec1.into_iter().zip(vec2).collect();

        Ok(vec)
    }

    async fn get_prev_double_occurrences(
        &self,
        index1: u64,
        index2: u64,
    ) -> Result<Vec<(u64, u64)>, Error> {
        let mut statement = self.connection.prepare(DOUBLE_PREV_QUERY)?;
        statement
            .bind_iter::<_, (_, i64)>([(":index1", index1 as i64), (":index2", index2 as i64)])?;

        let mut vec1: Vec<u64> = vec![];
        let mut vec2: Vec<u64> = vec![];
        while let Ok(sqlite::State::Row) = statement.next() {
            vec1.push(statement.read::<i64, _>("prev").unwrap() as u64);
        }

        if vec1.is_empty() {
            let err: Error = String::from("The length of vec is 0").into();
            return Err(err);
        }
        let prev = vec1[0];

        let mut statement = self.connection.prepare(REVERSE_QUERY)?;
        statement
            .bind_iter::<_, (_, i64)>([(":index1", prev as i64), (":index2", index1 as i64)])?;
        while let Ok(sqlite::State::Row) = statement.next() {
            vec2.push(statement.read::<i64, _>("occurrences").unwrap() as u64);
        }

        let vec = vec1.into_iter().zip(vec2).collect();

        Ok(vec)
    }
}

#[async_trait]
impl Blacklist for SqliteBlacklist {
    async fn blacklist(&self, chat_id: i64, user_id: u64) -> Result<(), Error> {
        let mut statement = self.connection.prepare(
            "INSERT OR IGNORE INTO Blacklist (chat_id, user_id) VALUES(:chat_id, :user_id);",
        )?;

        statement
            .bind_iter::<_, (_, i64)>([(":chat_id", chat_id), (":user_id", user_id as i64)])?;

        while let Ok(sqlite::State::Row) = statement.next() {}
        Ok(())
    }

    async fn unblacklist(&self, chat_id: i64, user_id: u64) -> Result<(), Error> {
        let mut statement = self
            .connection
            .prepare("DELETE FROM Blacklist WHERE chat_id = :chat_id AND user_id = :user_id;")?;

        statement
            .bind_iter::<_, (_, i64)>([(":chat_id", chat_id), (":user_id", user_id as i64)])?;

        while let Ok(sqlite::State::Row) = statement.next() {}
        Ok(())
    }

    async fn is_blacklisted(&self, chat_id: i64, user_id: u64) -> Result<bool, Error> {
        let mut statement = self
            .connection
            .prepare("SELECT 1 FROM Blacklist WHERE chat_id = :chat_id AND user_id = :user_id;")?;

        statement
            .bind_iter::<_, (_, i64)>([(":chat_id", chat_id), (":user_id", user_id as i64)])?;
        let mut is_blacklisted = false;

        while let Ok(sqlite::State::Row) = statement.next() {
            if let Ok(_) = statement.read::<i64, _>("1") {
                is_blacklisted = true;
            }
        }
        Ok(is_blacklisted)
    }
}
