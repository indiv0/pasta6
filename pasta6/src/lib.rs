pub use db::{get_all, insert, Database, Pool};
pub use handler::index;
pub use model::Todo;

/// An HTTP request.
pub struct Request;

/// An HTTP response.
#[must_use]
pub struct Response;

mod model {
    /// A representation of a Todo.
    #[derive(Debug, PartialEq)]
    pub struct Todo(pub String);

    impl Todo {
        /// Returns the content of the `Todo`.
        #[inline]
        pub fn content(&self) -> &str {
            &self.0
        }
    }
}

mod db {
    use crate::Todo;

    /// A handle to a database pool for storing all application data.
    #[derive(Clone)]
    pub struct Pool(deadpool_postgres::Pool);

    impl Pool {
        /// Configures and creates a new database connection pool.
        ///
        /// ```
        /// # use pasta6::*;
        /// let pool = Pool::new("postgres://test_user:test_password@localhost:5432/test_db?sslmode=disable");
        /// ```
        #[inline]
        pub fn new(database_url: &str) -> Self {
            // Parse the database URL into the configuration options.
            assert!(database_url.starts_with("postgres://"));
            let (_schema, database_url) = database_url.split_at(11);
            let (user, database_url) = database_url.split_at(database_url.find(':').unwrap());
            let (password, database_url) = database_url.split_at(database_url.find('@').unwrap());
            let (host, database_url) = database_url.split_at(database_url.find(':').unwrap());
            let (port, database_url) = database_url.split_at(database_url.find('/').unwrap());
            let (dbname, database_url) = database_url.split_at(database_url.find('?').unwrap());
            assert_eq!(database_url, "?sslmode=disable");
            let config = deadpool_postgres::Config {
                user: Some(user.to_string()),
                password: Some(password[1..].to_string()),
                host: Some(host[1..].to_string()),
                port: Some(port[1..].parse().unwrap()),
                dbname: Some(dbname[1..].to_string()),
                ..Default::default()
            };
            let pool = config.create_pool(None, tokio_postgres::NoTls).unwrap();
            Self(pool)
        }
    }

    impl Default for Pool {
        /// Configures and creates a new database connection pool.
        #[inline]
        fn default() -> Self {
            Self::new("postgres://test_user:test_password@127.0.0.1:5432/test_db?sslmode=disable")
        }
    }

    impl Pool {
        /// Returns a new database connection from the pool.
        #[inline]
        pub async fn get(&self) -> Database {
            Database(self.0.get().await.unwrap())
        }
    }

    /// A handle to a database connection.
    pub struct Database(deadpool_postgres::Client);

    /// Inserts a Todo into the database.
    ///
    /// ```
    /// # use pasta6::*;
    /// # let pool = Pool::default();
    /// # tokio_test::block_on(async move {
    /// # let db = pool.get().await;
    /// insert(&db, Todo("foo".to_string()));
    /// # });
    /// ```
    pub async fn insert(db: &Database, todo: Todo) -> Todo {
        const STATEMENT: &str = "INSERT INTO pasta.todo(content) VALUES ($1) RETURNING (content);";
        let statement = db.0.prepare(STATEMENT).await.unwrap();
        let row = db.0.query_one(&statement, &[&todo.0]).await.unwrap();
        Todo(row.get(0))
    }

    /// Gets all `Todo`s from the database.
    ///
    /// ```
    /// # use pasta6::*;
    /// # let pool = Pool::default();
    /// # tokio_test::block_on(async move {
    /// # let db = pool.get().await;
    /// get_all(&db);
    /// # });
    /// ```
    pub async fn get_all(db: &Database) -> impl Iterator<Item = Todo> {
        const STATEMENT: &str = "SELECT content FROM pasta.todo;";
        let statement = db.0.prepare(STATEMENT).await.unwrap();
        db.0.query(&statement, &[])
            .await
            .unwrap()
            .into_iter()
            .map(|row| Todo(row.get(0)))
    }
}

/// Entrypoint to the application.
pub fn run() {
    let (server, port) = start(8080);
    assert_eq!(port, 8080);
    server()
}

/// Creates the application (e.g., database connection pool, etc.) and
/// returns a function that runs the HTTP server to completion, as well as
/// the port the server bound to.
pub fn start(port: u16) -> (impl FnOnce(), u16) {
    // Initialize the server state.
    let pool =
        Pool::new("postgres://test_user:test_password@127.0.0.1:5432/test_db?sslmode=disable");
    // Initialize the HTTP server.
    server(pool, port)
}

/// Creates an HTTP server that runs on the provided port.
///
/// Returns a function that runs the HTTP server to completion, as well as
/// the port the HTTP server actually bound to.
fn server(pool: Pool, port: u16) -> (impl FnOnce(), u16) {
    use std::future::Future;
    use std::net::TcpListener;

    use actix_multipart::Multipart;
    use actix_web::get;
    use actix_web::http::header;
    use actix_web::http::StatusCode;
    use actix_web::post;
    use actix_web::web::Data;
    use actix_web::App;
    use actix_web::HttpResponse;
    use actix_web::HttpServer;
    use actix_web::Responder;
    use futures_util::stream::StreamExt;

    /// ```
    /// # use reqwest::{get, StatusCode};
    /// # let (server, port) = pasta6::start(0);
    /// let address = "http://127.0.0.1:8080";
    /// # let address = format!("http://127.0.0.1:{}", port);
    /// # std::thread::spawn(server);
    /// # let test = async move {
    /// assert_eq!(get(address).await?.status(), StatusCode::OK);
    /// # Ok::<_, reqwest::Error>(()) };
    /// # tokio_test::block_on(async { test.await.unwrap() });
    /// ```
    #[get("/")]
    async fn index(pool: Data<Pool>) -> impl Responder {
        HttpResponse::build(StatusCode::OK)
            .content_type("text/html; charset=utf-8")
            .body(handler::index(&pool.get().await).await)
    }

    /// ```
    /// # use reqwest::{Client, StatusCode, multipart::Form, redirect::Policy};
    /// # let (server, port) = pasta6::start(0);
    /// let address = "http://127.0.0.1:8080/todo";
    /// # let address = format!("http://127.0.0.1:{}/todo", port);
    /// # std::thread::spawn(server);
    /// # let test = async move {
    /// # let post = |a| Client::builder().redirect(Policy::none()).build().unwrap().post(a);
    /// let form = Form::new().text("content", "foo");
    /// let response = post(address).multipart(form).send().await?;
    /// assert_eq!(response.status(), StatusCode::SEE_OTHER);
    /// assert_eq!(response.headers().get("location").unwrap().to_str().unwrap(), "/");
    /// # Ok::<_, reqwest::Error>(()) };
    /// # tokio_test::block_on(async { test.await.unwrap() });
    /// ```
    #[post("/todo")]
    async fn add_todo(pool: Data<Pool>, mut payload: Multipart) -> impl Responder {
        // Allocate a vector of bytes to store the value of the `content` key.
        let mut bytes = vec![];
        // Iterate over multipart stream until we find the `content` key.
        let mut content = None;
        while let Some(Ok(mut field)) = payload.next().await {
            let disposition = field.content_disposition();
            let field_name = disposition.get_name().unwrap();
            if field_name != "content" {
                continue;
            }
            // Fields are streams of `Bytes` objects.
            // Collect the streams into a single value.
            // TODO: test what happens if the field value is way too long.
            while let Some(chunk) = field.next().await {
                bytes.extend_from_slice(&chunk.unwrap());
            }
            // Parse the bytes to a UTF-8 `String`.
            // TODO: sanitize the string as well.
            content = Some(std::str::from_utf8(&bytes).unwrap());
            break;
        }
        handler::add_todo(&pool.get().await, content.unwrap().to_string()).await;
        HttpResponse::SeeOther()
            .insert_header((header::LOCATION, "/"))
            .finish()
    }

    // Bind the TCP listener on the requested port.
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(Data::new(pool.clone()))
            .service(index)
            .service(add_todo)
    })
    .listen(listener)
    .unwrap()
    .run();

    #[actix_web::main]
    async fn serve_(server: impl Future<Output = Result<(), std::io::Error>>) {
        server.await.unwrap()
    }

    (move || serve_(server), port)
}

mod handler {
    use crate::Database;
    use crate::Todo;
    use crate::{get_all, insert};

    /// Serializes a `GET /` response into a string slice.
    pub async fn index(db: &Database) -> String {
        // Fetch a list of all TODOs from the database.
        // TODO: limit this to the 10 newest TODOs?
        let mut todos = get_all(db).await.peekable();

        // Serialize the TODOs to HTML.
        let todos = if todos.peek().is_some() {
            let mut string = String::new();
            string.push_str("<ul>");
            for todo in todos {
                string.push_str("<li>");
                string.push_str(todo.content());
                string.push_str("</li>");
            }
            string.push_str("</ul>");
            string
        } else {
            String::new()
        };

        format!(
            "<html>\
            <head></head>\
            <body>\
            <form method=\"post\" action=\"/todo\" enctype=\"multipart/form-data\">\
              <label for=\"content\">TODO:</label>\
              <input type=\"text\" name=\"content\" id=\"content\" required>\
              {}\
            </form>\
            </body>\
            </html>",
            todos
        )
    }

    /// Serializes a `POST /todo` response into a string slice.
    pub async fn add_todo(db: &Database, content: String) {
        // Insert the TODO into the database.
        insert(db, Todo(content)).await;
    }
}
