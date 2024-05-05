use std::io::{Read, Write};
use std::net::{TcpListener};
use sqlite::{State, Connection};

#[macro_use]
extern crate serde_derive;

const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_SERVER_ERROR: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: Option<i64>,
    name: String,
    age: i64,
}


fn main() {
    let listener = TcpListener::bind("0.0.0.0:3000").unwrap();
    let connection = init_database();
    handle_stream(listener, &connection)
}


fn init_database() -> Connection {
    let connection = sqlite::open(":memory:").unwrap();
    let query = "
        CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, age INTEGER);
        INSERT INTO users (name, age) VALUES ('Bara', 22);
        INSERT INTO users (name, age) VALUES ('Bama', 22);
    ";
    connection.execute(query).unwrap();
    connection
}

fn handle_stream(listener: TcpListener, db: &Connection) {
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buffer = [0; 1024];
                let mut request = String::new();

                match stream.read(&mut buffer) {
                    Ok(size) => {
                        request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());
                        let (status, content) = handle_routes(&request, db);
                        stream.write_all(format!("{}{}", status, content).as_bytes()).unwrap();
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                        stream.write_all(format!("{}", INTERNAL_SERVER_ERROR).as_bytes()).unwrap();
                    }
                }

            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}

fn handle_routes(request: &String, db: &Connection) -> (String, String) {
    match request {
        r if r.starts_with("POST /users") => post_user(db, request.clone()),
        r if r.starts_with("GET /users") => get_users(db),
        r if r.starts_with("PUT /user/") => put_user(db, request.clone()),
        r if r.starts_with("DELETE /user/") => delete_user(db, request.clone()),
        _ => (NOT_FOUND.to_string(), "404 Not Found".to_string()),
    }
}

fn get_users(db: &Connection) -> (String, String) {
    let query = "SELECT * FROM users";
    let mut statement = db.prepare(query).unwrap();
    let mut result: Vec<User> = vec![];

    while let Ok(State::Row) = statement.next() {
        let name = statement.read::<String, _>("name").unwrap();
        let age = statement.read::<i64, _>("age").unwrap();
        let id = statement.read::<i64, _>("id").unwrap();
        result.push(User {
            name,
            age,
            id: Some(id)
        });
    }
    (OK_RESPONSE.to_string(), serde_json::to_string(&result).unwrap())
}

fn post_user(db: &Connection, request: String) -> (String, String) {
    let user = get_user_request_body(request.clone()).unwrap();
    let query = format!("INSERT INTO users (name, age) VALUES ('{}', '{}')", user.name, user.age);
    let mut statement = db.prepare(query).unwrap();
    statement.iter().next();
    (OK_RESPONSE.to_string(), "".to_string())
}

fn put_user(db: &Connection, request: String) -> (String, String) {
    let id = get_id(&request).parse::<i32>().unwrap();
    let user = get_user_request_body(request.clone()).unwrap();
    let query = format!("UPDATE users SET name = '{}', age = {} WHERE id = {}", user.name, user.age, id);
    let mut statement = db.prepare(query).unwrap();
    statement.iter().next();
    (OK_RESPONSE.to_string(), "".to_string())
}

fn delete_user(db: &Connection, request: String) -> (String, String) {
    let id = get_id(&request).parse::<i32>().unwrap();
    let query = format!("DELETE FROM users WHERE id = {}", id);
    let mut statement = db.prepare(query).unwrap();
    statement.iter().next();
    (OK_RESPONSE.to_string(), "".to_string())
}

fn get_user_request_body(request: String) -> Result<User, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}

fn get_id(request: &str) -> &str {
    request.split("/").nth(2).unwrap_or_default().split_whitespace().next().unwrap_or_default()
}
