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

const SINGLE_NEXT_QUERY: &str = "
    SELECT * FROM Occurrence WHERE curr = :index2;
    ";

const DOUBLE_NEXT_QUERY: &str = "
    SELECT * FROM Occurrence WHERE prev = :index1 AND curr = :index2;
    ";

type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct SqliteDB {
    database: sqlite::Connection,
}

impl SqliteDB {
    pub fn new(path: &std::path::Path, flags: sqlite::OpenFlags) -> Result<Self, Error> {
        let db = SqliteDB {
            database: sqlite::Connection::open_with_flags(path, flags)?,
        };
        db.database.execute(INIT_QUERY)?;
        Ok(db)
    }
}

pub trait Database {
    fn add_word(&self, tuple: (&str, &str)) -> Result<u64, Error>;

    fn increment(&self, index1: u64, index2: u64, index3: u64) -> Result<(), Error>;

    fn get_word(&self, index: u64) -> Result<String, Error>;

    fn get_single_occurrences(&self, index: u64) -> Result<Vec<(u64, u64)>, Error>;

    fn get_double_occurrences(&self, index1: u64, index2: u64) -> Result<Vec<(u64, u64)>, Error>;
}

impl Database for SqliteDB {
    fn add_word(&self, tuple: (&str, &str)) -> Result<u64, Error> {
        let mut statement = self.database.prepare(ADD_QUERY)?;

        statement.bind_iter::<_, (_, sqlite::Value)>([
            (":keyword", tuple.0.clone().into()),
            (":string", tuple.1.clone().into()),
        ])?;

        while let Ok(sqlite::State::Row) = statement.next() {}

        let mut statement = self
            .database
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

    fn increment(&self, index1: u64, index2: u64, index3: u64) -> Result<(), Error> {
        let mut statement = self.database.prepare(INCREMENT_QUERY)?;
        statement.bind_iter::<_, (_, i64)>([
            (":index1", index1 as i64),
            (":index2", index2 as i64),
            (":index3", index3 as i64),
        ])?;
        while let Ok(sqlite::State::Row) = statement.next() {}
        Ok(())
    }

    fn get_word(&self, index: u64) -> Result<String, Error> {
        let mut statement = self.database.prepare(GET_QUERY)?;
        statement.bind((":id", index as i64))?;

        if let Ok(sqlite::State::Row) = statement.next() {
            Ok(statement.read::<String, _>("string")?)
        } else {
            let err: Error =
                String::from("None was returned. Is your file corrupted or missing?").into();
            Err(err)
        }
    }

    fn get_single_occurrences(&self, index: u64) -> Result<Vec<(u64, u64)>, Error> {
        let mut statement = self.database.prepare(SINGLE_NEXT_QUERY)?;
        statement.bind((":index2", index as i64))?;

        let mut vec: Vec<(u64, u64)> = vec![];
        while let Ok(sqlite::State::Row) = statement.next() {
            vec.push((
                statement.read::<i64, _>("next").unwrap() as u64,
                statement.read::<i64, _>("occurrences").unwrap() as u64,
            ));
        }
        Ok(vec)
    }

    fn get_double_occurrences(&self, index1: u64, index2: u64) -> Result<Vec<(u64, u64)>, Error> {
        let mut statement = self.database.prepare(DOUBLE_NEXT_QUERY)?;
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
}
