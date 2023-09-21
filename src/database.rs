use sqlite;

pub enum DatabaseMessage {
    AddWord(String, String),
    Increment(usize, usize),
    GetWord(usize),
    GetNextWords(usize),
}

pub enum DatabaseResult {
    None,
    Int(usize),
    String(String),
    VecTuple(Vec<(usize, usize)>),
}

const INIT_QUERY: &str = "
    CREATE TABLE IF NOT EXISTS Words(
        id INTEGER PRIMARY KEY,
        keyword VARCHAR(20),
        string VARCHAR(255),
        UNIQUE(keyword,string)
    );

    CREATE TABLE IF NOT EXISTS Occurrence (
        index1 INT NOT NULL,
        index2 INT NOT NULL,
        occurrences INT,
        UNIQUE(index1, index2)
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
    INSERT INTO Occurrence (index1, index2, occurrences) VALUES(:index1, :index2, 1)
    ON CONFLICT(index1, index2) DO UPDATE SET occurrences = occurrences + 1;
";

const GET_QUERY: &str = "
        SELECT string FROM Words WHERE id = :id;
";

const GET_NEXT_QUERY: &str = "
         SELECT * FROM Occurrence WHERE index1 = :index1;
";

pub fn database(
    database: String,
    message: DatabaseMessage,
) -> Result<DatabaseResult, Box<dyn std::error::Error>> {
    let flags = sqlite::OpenFlags::new()
        .set_create()
        .set_full_mutex()
        .set_read_write();
    let path_name = &format!("./{}.db", database);
    let path = std::path::Path::new(path_name);
    let connection = sqlite::Connection::open_with_flags(path, flags)?;

    connection.execute(INIT_QUERY)?;

    match message {
        DatabaseMessage::AddWord(a, b) => {
            let mut statement = connection.prepare(ADD_QUERY)?;
            statement.bind_iter::<_, (_, sqlite::Value)>([
                (":keyword", a.clone().into()),
                (":string", b.clone().into()),
            ])?;

            while let Ok(sqlite::State::Row) = statement.next() {}

            let mut statement = connection
                .prepare("SELECT id FROM Words WHERE keyword = :keyword AND string = :string")?;
            statement.bind_iter::<_, (_, sqlite::Value)>([
                (":keyword", a.into()),
                (":string", b.into()),
            ])?;

            let mut id: i64 = 0;
            while let Ok(sqlite::State::Row) = statement.next() {
                id = statement.read::<i64, _>("id").unwrap();
            }

            return Ok(DatabaseResult::Int(id as usize));
        }
        DatabaseMessage::Increment(a, b) => {
            let mut statement = connection.prepare(INCREMENT_QUERY)?;
            statement.bind_iter::<_, (_, i64)>([(":index1", a as i64), (":index2", b as i64)])?;
            while let Ok(sqlite::State::Row) = statement.next() {}
        }
        DatabaseMessage::GetWord(a) => {
            let mut statement = connection.prepare(GET_QUERY)?;
            statement.bind((":id", a as i64))?;

            if let Ok(sqlite::State::Row) = statement.next() {
                return Ok(DatabaseResult::String(
                    statement.read::<String, _>("string").unwrap(),
                ));
            }
        }
        DatabaseMessage::GetNextWords(a) => {
            let mut statement = connection.prepare(GET_NEXT_QUERY)?;
            statement.bind((":index1", a as i64))?;

            let mut vec: Vec<(usize, usize)> = vec![];
            while let Ok(sqlite::State::Row) = statement.next() {
                vec.push((
                    statement.read::<i64, _>("index2").unwrap() as usize,
                    statement.read::<i64, _>("occurrences").unwrap() as usize,
                ));
            }
            return Ok(DatabaseResult::VecTuple(vec));
        }
    };

    Ok(DatabaseResult::None)
}
