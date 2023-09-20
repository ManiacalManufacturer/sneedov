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
    VecTuple(Vec<(usize, usize)>),
}

// const PATH_TO_DATABASE: &str = "./database/{}/{}.db";

const INIT_QUERY: &str = "
    CREATE TABLE IF NOT EXISTS Words(
        id INTEGER PRIMARY KEY,
        keyword VARCHAR(20),
        string VARCHAR(255)
    );
";

// const ADD_QUERY: &str = "
//     INSERT OR REPLACE INTO Words (keyword, string) VALUES(
//         :keyword,
//         :string
//     );
// ";

// const CREATE_QUERY: &str = "
//     CREATE TABLE IF NOT EXISTS Word{}(
//         id INT PRIMARY KEY,
//         occurrences INT
//     );
// ";

// const INCREMENT_QUERY: &str = "
//         UPDATE Word:current SET occurrences = occurrences + 1
//         WHERE id = :next;
// ";

const GET_QUERY: &str = "
        SELECT string FROM Words WHERE id = ?;
";

// const GET_NEXT_QUERY: &str = "
//         SELECT * FROM Word?;
// ";

pub async fn database(
    _atabase: String,
    message: DatabaseMessage,
) -> Result<DatabaseResult, Box<dyn std::error::Error>> {
    let flags = sqlite::OpenFlags::new()
        .set_create()
        .set_full_mutex()
        .set_read_write();
    let path = std::path::Path::new("./test.db");
    let connection = sqlite::Connection::open_with_flags(path, flags)?;

    connection.execute(INIT_QUERY)?;
    // let query = String::new();

    match message {
        //TODO: Create a table for the new word
        DatabaseMessage::AddWord(a, b) => {
            // println!("test {} {}", a, b);
            // let mut statement = connection.prepare(ADD_QUERY)?;
            // statement.bind_iter::<_, (_, sqlite::Value)>([
            //     (":keyword", a.clone().into()),
            //     (":string", b.clone().into()),
            // ])?;

            let format = format!(
                "
                    UPSERT INTO Words (id, keyword, string) VALUES(
                        null,
                        \"{}\",
                        \"{}\"
                );
                ",
                a.clone(),
                b.clone()
            );
            connection.execute(format)?;

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

            //let mut statement = connection.prepare(CREATE_QUERY)?;
            //statement.bind((":id", id))?;

            let format = format!(
                "
                CREATE TABLE IF NOT EXISTS Word{}(
                    id INTEGER PRIMARY KEY,
                    occurrences INT
                );
            ",
                id
            );
            connection.execute(format)?;
            return Ok(DatabaseResult::Int(id as usize));
        }
        DatabaseMessage::Increment(a, b) => {
            //TODO: Add increment
            let format = format!(
                "
                INSERT INTO Word{} (id, occurrence) VALUES({}, 1)
                ON CONFLICT(id, occurrence) DO UPDATE SET occurrence = occurence + 1;
            ",
                a, b
            );
            connection.execute(format)?;
        }
        DatabaseMessage::GetWord(a) => {
            //TODO: Add get word
            let mut statement = connection.prepare(GET_QUERY)?;
            statement.bind((1, a as i64))?;

            if let Ok(sqlite::State::Row) = statement.next() {
                return Ok(DatabaseResult::Int(
                    statement.read::<i64, _>("id").unwrap() as usize
                ));
            }
        }
        DatabaseMessage::GetNextWords(a) => {
            //TODO: Add get next words
            let mut statement = connection.prepare(GET_QUERY)?;
            statement.bind((1, a as i64))?;

            let mut vec: Vec<(usize, usize)> = vec![];
            while let Ok(sqlite::State::Row) = statement.next() {
                vec.push((
                    statement.read::<i64, _>("id").unwrap() as usize,
                    statement.read::<i64, _>("occurrence").unwrap() as usize,
                ));
            }
            return Ok(DatabaseResult::VecTuple(vec));
        }
    };

    //TODO: Add executing query here
    Ok(DatabaseResult::None)
}
