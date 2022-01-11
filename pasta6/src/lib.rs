use lunatic::net::{TcpListener, TcpStream};
use lunatic::{process, Mailbox};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::io::{self, BufRead, BufReader, BufWriter, Empty, Read, Write};

/// Entrypoint to the application.
pub fn run() {
    let mailbox = unsafe { Mailbox::<()>::new() };
    run_(mailbox);
}

fn run_(_mailbox: Mailbox<()>) {
    let listener = TcpListener::bind("0.0.0.0:9090").unwrap();
    while let Ok((tcp_stream, _peer)) = listener.accept() {
        // Handle connections in a new process.
        process::spawn_with(tcp_stream, handle_tcp_stream).unwrap();
    }

    //let (server, port) = start(8080);
    //assert_eq!(port, 8080);
    //server()
}

fn handle_tcp_stream(mut tcp_stream: TcpStream, mailbox: Mailbox<()>) {
    // TODO: "it is critical to call flush before BufWriter is dropped".
    let mut connection = Connection::new(tcp_stream).unwrap();
    let mut request_headers_map = HashMap::new();
    loop {
        // TODO: can this buffer be preserved between requests?
        let request = if let Some(request) =
            Connection::parse_request_head(&mut connection.reader, &mut request_headers_map)
                .unwrap()
        {
            request
        } else {
            // EOF reached, so connection closed.
            return;
        };
        //println!("received request: {:?}", request);
        // If the request has a Content-Length, consume that many bytes for
        // the body.
        let content_length: usize = request
            .headers
            .get("Content-Length")
            // TODO: figure out proper size rather than usize here
            // TODO: remove this extra UTF-8 cast and parse directly to int.
            .map(|v| std::str::from_utf8(v.as_slice()).unwrap())
            .map(|s| s.parse().unwrap())
            .unwrap_or(0);
        //println!("reading {} bytes of request body", content_length);
        let mut request_body = vec![0; content_length];
        connection.reader.read_exact(&mut request_body).unwrap();
        // Respond to the request.
        let mut body = Body::empty();
        let mut response = Response::new(200, &[]);
        write_response(&mut connection.writer, &mut response, &mut body).unwrap();
    }
}

#[derive(Debug)]
struct Request<'headers> {
    method: Method,
    path: String,
    // TODO: use our own internal header type here to avoid type leakage.
    headers: &'headers mut HashMap<String, Vec<u8>>,
}

#[derive(Debug)]
struct Response<'headers, 'header> {
    code: u16,
    headers: &'headers [httparse::Header<'header>],
}

impl<'headers, 'header> Response<'headers, 'header> {
    fn new(code: u16, headers: &'headers [httparse::Header<'header>]) -> Self {
        Self { code, headers }
    }
}

struct Body<R>
where
    R: Read,
{
    len: usize,
    reader: R,
}

impl Body<Empty> {
    fn empty() -> Self {
        Self {
            len: 0,
            reader: io::empty(),
        }
    }
}

fn write_response<R>(
    writer: &mut impl Write,
    // TODO: should we consume the `Response` since we're consuming the
    // `Body` reader?
    response: &mut Response,
    body: &mut Body<R>,
) -> Result<(), io::Error>
where
    R: Read,
{
    write!(writer, "HTTP/1.1 {}\r\n", response.code)?;
    let mut content_length_included = false;
    for header in response.headers {
        // Content-Length is a single value header, so we fail to write the
        // response unless both values match, in which case we still only
        // write one.
        assert!(header.name.is_ascii());
        assert!(header.name.trim() == header.name);
        if header.name.to_ascii_lowercase() == "content-length" {
            assert!(header.value == body.len.to_be_bytes());
            content_length_included = true;
        }
        write!(writer, "{}: ", header.name)?;
        writer.write_all(header.value)?;
        write!(writer, "\r\n")?;
    }
    // If the user did not specify a content-length, we write one ourselves.
    if !content_length_included {
        write!(writer, "Content-Length: {}\r\n", body.len)?;
    }
    write!(writer, "\r\n")?;
    io::copy(&mut body.reader, writer)?;
    Ok(())
}

#[derive(Debug)]
enum ParseError {
    Io(io::Error),
    HeadTooLong,
    ParseError(httparse::Error),
}

struct Connection {
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
}

impl Connection {
    fn new(tcp_stream: TcpStream) -> Result<Self, io::Error> {
        let mut reader = BufReader::new(tcp_stream.clone());
        let writer = BufWriter::new(tcp_stream);
        // Read from the internal buffer at least once, because the parse
        // methods assume that an empty internal buffer means the stream has
        // reached EOF.
        reader.fill_buf()?;
        Ok(Self { reader, writer })
    }

    fn parse_request_head<'headers>(
        reader: &mut BufReader<TcpStream>,
        headers_map: &'headers mut HashMap<String, Vec<u8>>,
    ) -> Result<Option<Request<'headers>>, ParseError> {
        let mut headers = [httparse::EMPTY_HEADER; 16];
        'outer: loop {
            'inner: loop {
                // Get a reference to the internally buffered data.
                let buffer = reader.buffer();

                // If the buffer is empty, then the stream has reached EOF.
                if buffer.is_empty() {
                    return Ok(None);
                }

                // Need at least 3 bytes to parse the header.
                if buffer.len() < 3 {
                    break 'inner;
                }

                // Find the double CRLF that indicates header end.
                let mut newlines = memchr::memchr_iter(b'\n', &buffer[3..]);
                let double_clrf = newlines.find(|idx| &buffer[idx - 3..=*idx] == b"\r\n\r\n");
                if double_clrf.is_some() {
                    break 'inner;
                }

                break 'outer;
            }

            // If we need more data to parse the headers, but the internal
            // buffer is at capacity, then the request is too long to be
            // parsed.
            if reader.buffer().len() == reader.capacity() {
                return Err(ParseError::HeadTooLong);
            }

            // Read some more data, then repeat.
            reader.fill_buf().map_err(ParseError::Io)?;
        }

        // Parse the request headers.
        //println!("{}", std::str::from_utf8(reader.buffer()).unwrap());
        //println!("parsing request from {} bytes", reader.buffer().len());
        let mut request = httparse::Request::new(&mut headers);
        match request
            .parse(reader.buffer())
            .map_err(ParseError::ParseError)?
        {
            // FIXME: this case should be unreachable because we found the
            // double CLRF already.
            httparse::Status::Partial => panic!("more data needed"),
            httparse::Status::Complete(body_idx) => {
                // Convert the `httparse::Request` to a `Request`.
                headers_map.clear();
                for header in &*request.headers {
                    headers_map.insert(header.name.to_string(), header.value.to_vec());
                }
                let req = Request::new(
                    Method::from_str(request.method.unwrap()),
                    request.path.unwrap().to_string(),
                    headers_map,
                );
                reader.consume(body_idx);
                Ok(Some(req))
            }
        }
        //let mut headers = [httparse::EMPTY_HEADER; 16];
        //let mut request = httparse::Request::new(&mut headers);
        //// TODO: what happens if the request doesn't fit in 4096 bytes?
        //let mut buffer = [0; 4096];
        //let mut bytes_read;
        //loop {
        //    // Read as many bytes as possible.
        //    bytes_read = buf_reader.read(&mut buffer).unwrap();
        //    // If we've reached EOF, there's nothing left to do.
        //    if bytes_read == 0 {
        //        return;
        //    }

        //    // Search any newly read bytes for newline characters.
        //    let newlines = memchr::memchr_iter(b'\n', &buffer[..bytes_read]);
        //    // Filter out any newlines that start at the beginning of the
        //    // buffer since they can't possibly have a carriage return
        //    // preceding them.
        //    let newlines = newlines.filter(|idx| *idx != 0);
        //    // For each newline, check if the byte before it is a carriage
        //    // return. If it is, then that newline is part of a CRLF.
        //    let mut crlfs = newlines.filter(|idx| buffer[idx - 1] == b'\r');
        //    // If there exists a CRLF in the buffer, then we can attempt to
        //    // parse the request.
        //    if crlfs.next().is_some() {
        //        break;
        //    }
        //}
        //// If the request can be parsed, then we can split off the
        //// part of the buffer containing the request.
        //match request.parse(&buffer[..bytes_read]) {
        //    Ok(httparse::Status::Complete(_body_idx)) => {}
        //    Ok(httparse::Status::Partial) => {}
        //    Err(error) => panic!("{}", error),
        //}
        ////let _bytes_written = tcp_stream.write(&buffer[..]).unwrap();
    }

    fn parse_response_head<'reader, 'headers, 'header>(
        &'reader mut self,
        headers: &'headers mut [httparse::Header<'header>; 16],
    ) -> Result<Option<Response<'headers, 'header>>, ParseError>
    where
        'reader: 'header,
    {
        'outer: loop {
            'inner: loop {
                // Get a reference to the internally buffered data.
                let buffer = self.reader.buffer();

                // If the buffer is empty, then the stream has reached EOF.
                if buffer.is_empty() {
                    return Ok(None);
                }

                // Need at least 3 bytes to parse the header.
                if buffer.len() < 3 {
                    break 'inner;
                }

                // Find the double CRLF that indicates header end.
                let mut newlines = memchr::memchr_iter(b'\n', &buffer[3..]);
                let double_clrf = newlines.find(|idx| &buffer[idx - 3..=*idx] == b"\r\n\r\n");
                if double_clrf.is_some() {
                    break 'inner;
                }

                break 'outer;
            }

            // If we need more data to parse the headers, but the internal
            // buffer is at capacity, then the request is too long to be
            // parsed.
            if self.reader.buffer().len() == self.reader.capacity() {
                return Err(ParseError::HeadTooLong);
            }

            // Read some more data, then repeat.
            self.reader.fill_buf().map_err(ParseError::Io)?;
        }

        // Parse the response headers.
        //let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut response = httparse::Response::new(headers);
        match response
            .parse(self.reader.buffer())
            .map_err(ParseError::ParseError)?
        {
            // FIXME: this case should be unreachable because we found the
            // double CLRF already.
            httparse::Status::Partial => panic!("more data needed"),
            httparse::Status::Complete(_body_idx) => {
                // Convert the `httparse::Response` to a `Response`.
                let resp = Response::new(response.code.unwrap(), response.headers);
                return Ok(Some(resp));
            }
        }
    }
}

#[derive(Debug)]
enum Method {
    Get,
}

impl Display for Method {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Method::Get => write!(f, "GET"),
        }
    }
}

impl Method {
    fn from_str(string: &str) -> Self {
        match string {
            "GET" => Method::Get,
            _ => unimplemented!(),
        }
    }
}

struct Header<'a> {
    name: &'a str,
    value: &'a [u8],
}

impl<'headers> Request<'headers> {
    fn new(method: Method, path: String, headers: &'headers mut HashMap<String, Vec<u8>>) -> Self {
        Self {
            method,
            path,
            headers,
        }
    }
}

fn write_request(
    writer: &mut impl Write,
    request: &Request,
    body: &mut impl Read,
) -> Result<(), io::Error> {
    write!(writer, "{} {} HTTP/1.1\r\n", request.method, request.path,)?;
    for (name, value) in request.headers.iter() {
        write!(writer, "{}: ", name)?;
        writer.write_all(value)?;
        write!(writer, "\r\n")?;
    }
    write!(writer, "\r\n")?;
    io::copy(body, writer)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::*;
    use lunatic::process::{self, sleep};
    use lunatic::{Config, Environment};
    use std::io::Empty;

    #[test]
    fn test() {
        let _proc = process::spawn(run_).unwrap();
        //let mailbox = unsafe { Mailbox::<()>::new() };
        //let mut config = Config::new(10_000, Some(u64::MAX));
        //config.allow_namespace("");
        //let mut environment = Environment::new(config).unwrap();
        //let module = environment.add_this_module().unwrap();
        //module
        //    .spawn_link(mailbox, |_parent: Mailbox<()>| {
        //let rt = tokio::runtime::Runtime::new().unwrap();
        let mut tcp_stream = TcpStream::connect("127.0.0.1:9090").unwrap();

        // Send an HTTP request to the server.
        let mut headers = HashMap::new();
        headers.insert("Accept".to_string(), b"*/*".to_vec());
        let request = Request::new(Method::Get, "/".to_owned(), &mut headers);
        let body = Vec::new();
        let mut body_reader = BufReader::new(&body[..]);
        write_request(&mut tcp_stream, &request, &mut body_reader).unwrap();

        // Receive an HTTP response from the server.
        let mut connection = Connection::new(tcp_stream).unwrap();
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let response = connection
            .parse_response_head(&mut headers)
            .unwrap()
            .unwrap();
        assert_eq!(response.code, 200);

        //tcp_stream.write_all();
        //let raw_module =
        //    })
        //    .unwrap();
        //sleep(1000);
    }
}

//pub use db::{get_100, insert, Database, Pool};
//pub use handler::index;
//pub use model::Todo;
//
///// An HTTP request.
//pub struct Request;
//
///// An HTTP response.
//#[must_use]
//pub struct Response;
//
//mod model {
//    /// A representation of a Todo.
//    #[derive(Debug, PartialEq)]
//    pub struct Todo(pub String);
//
//    impl Todo {
//        /// Returns the content of the `Todo`.
//        #[inline]
//        pub fn content(&self) -> &str {
//            &self.0
//        }
//    }
//}
//
//mod db {
//    use crate::Todo;
//
//    /// A handle to a database pool for storing all application data.
//    #[derive(Clone)]
//    pub struct Pool(deadpool_postgres::Pool);
//
//    impl Pool {
//        /// Configures and creates a new database connection pool.
//        ///
//        /// ```
//        /// # use pasta6::*;
//        /// let pool = Pool::new("postgres://test_user:test_password@localhost:5432/test_db?sslmode=disable");
//        /// ```
//        #[inline]
//        pub fn new(database_url: &str) -> Self {
//            // Parse the database URL into the configuration options.
//            assert!(database_url.starts_with("postgres://"));
//            let (_schema, database_url) = database_url.split_at(11);
//            let (user, database_url) = database_url.split_at(database_url.find(':').unwrap());
//            let (password, database_url) = database_url.split_at(database_url.find('@').unwrap());
//            let (host, database_url) = database_url.split_at(database_url.find(':').unwrap());
//            let (port, database_url) = database_url.split_at(database_url.find('/').unwrap());
//            let (dbname, database_url) = database_url.split_at(database_url.find('?').unwrap());
//            assert_eq!(database_url, "?sslmode=disable");
//            let config = deadpool_postgres::Config {
//                user: Some(user.to_string()),
//                password: Some(password[1..].to_string()),
//                host: Some(host[1..].to_string()),
//                port: Some(port[1..].parse().unwrap()),
//                dbname: Some(dbname[1..].to_string()),
//                ..Default::default()
//            };
//            let pool = config.create_pool(None, tokio_postgres::NoTls).unwrap();
//            Self(pool)
//        }
//    }
//
//    impl Default for Pool {
//        /// Configures and creates a new database connection pool.
//        #[inline]
//        fn default() -> Self {
//            Self::new("postgres://test_user:test_password@127.0.0.1:5432/test_db?sslmode=disable")
//        }
//    }
//
//    impl Pool {
//        /// Returns a new database connection from the pool.
//        #[inline]
//        pub async fn get(&self) -> Database {
//            Database(self.0.get().await.unwrap())
//        }
//    }
//
//    /// A handle to a database connection.
//    pub struct Database(deadpool_postgres::Client);
//
//    /// Inserts a Todo into the database.
//    ///
//    /// ```
//    /// # use pasta6::*;
//    /// # let pool = Pool::default();
//    /// # tokio_test::block_on(async move {
//    /// # let db = pool.get().await;
//    /// insert(&db, Todo("foo".to_string()));
//    /// # });
//    /// ```
//    pub async fn insert(db: &Database, todo: Todo) -> Todo {
//        const STATEMENT: &str = "INSERT INTO pasta.todo(content) VALUES ($1) RETURNING (content);";
//        let statement = db.0.prepare(STATEMENT).await.unwrap();
//        let row = db.0.query_one(&statement, &[&todo.0]).await.unwrap();
//        Todo(row.get(0))
//    }
//
//    /// Get 100 `Todo`s from the database.
//    ///
//    /// ```
//    /// # use pasta6::*;
//    /// # let pool = Pool::default();
//    /// # tokio_test::block_on(async move {
//    /// # let db = pool.get().await;
//    /// get_100(&db);
//    /// # });
//    /// ```
//    pub async fn get_100(db: &Database) -> impl Iterator<Item = Todo> {
//        const STATEMENT: &str = "SELECT content FROM pasta.todo LIMIT 100;";
//        let statement = db.0.prepare(STATEMENT).await.unwrap();
//        db.0.query(&statement, &[])
//            .await
//            .unwrap()
//            .into_iter()
//            .map(|row| Todo(row.get(0)))
//    }
//}
//
///// Creates the application (e.g., database connection pool, etc.) and
///// returns a function that runs the HTTP server to completion, as well as
///// the port the server bound to.
//pub fn start(port: u16) -> (impl FnOnce(), u16) {
//    // Initialize the server state.
//    let pool =
//        Pool::new("postgres://test_user:test_password@127.0.0.1:5432/test_db?sslmode=disable");
//    // Initialize the HTTP server.
//    server(pool, port)
//}
//
///// Creates an HTTP server that runs on the provided port.
/////
///// Returns a function that runs the HTTP server to completion, as well as
///// the port the HTTP server actually bound to.
//fn server(pool: Pool, port: u16) -> (impl FnOnce(), u16) {
//    use std::future::Future;
//    use std::net::TcpListener;
//
//    use actix_multipart::Multipart;
//    use actix_web::get;
//    use actix_web::http::header;
//    use actix_web::http::StatusCode;
//    use actix_web::post;
//    use actix_web::web::Data;
//    use actix_web::App;
//    use actix_web::HttpResponse;
//    use actix_web::HttpServer;
//    use actix_web::Responder;
//    use futures_util::stream::StreamExt;
//
//    /// ```
//    /// # use reqwest::{get, StatusCode};
//    /// # let (server, port) = pasta6::start(0);
//    /// let address = "http://127.0.0.1:8080";
//    /// # let address = format!("http://127.0.0.1:{}", port);
//    /// # std::thread::spawn(server);
//    /// # let test = async move {
//    /// assert_eq!(get(address).await?.status(), StatusCode::OK);
//    /// # Ok::<_, reqwest::Error>(()) };
//    /// # tokio_test::block_on(async { test.await.unwrap() });
//    /// ```
//    #[get("/")]
//    async fn index(pool: Data<Pool>) -> impl Responder {
//        HttpResponse::build(StatusCode::OK)
//            .content_type("text/html; charset=utf-8")
//            .body(handler::index(&pool.get().await).await)
//    }
//
//    /// ```
//    /// # use reqwest::{Client, StatusCode, multipart::Form, redirect::Policy};
//    /// # let (server, port) = pasta6::start(0);
//    /// let address = "http://127.0.0.1:8080/todo";
//    /// # let address = format!("http://127.0.0.1:{}/todo", port);
//    /// # std::thread::spawn(server);
//    /// # let test = async move {
//    /// # let post = |a| Client::builder().redirect(Policy::none()).build().unwrap().post(a);
//    /// let form = Form::new().text("content", "foo");
//    /// let response = post(address).multipart(form).send().await?;
//    /// assert_eq!(response.status(), StatusCode::SEE_OTHER);
//    /// assert_eq!(response.headers().get("location").unwrap().to_str().unwrap(), "/");
//    /// # Ok::<_, reqwest::Error>(()) };
//    /// # tokio_test::block_on(async { test.await.unwrap() });
//    /// ```
//    #[post("/todo")]
//    async fn add_todo(pool: Data<Pool>, mut payload: Multipart) -> impl Responder {
//        // Allocate a vector of bytes to store the value of the `content` key.
//        let mut bytes = vec![];
//        // Iterate over multipart stream until we find the `content` key.
//        let mut content = None;
//        while let Some(Ok(mut field)) = payload.next().await {
//            let disposition = field.content_disposition();
//            let field_name = disposition.get_name().unwrap();
//            if field_name != "content" {
//                continue;
//            }
//            // Fields are streams of `Bytes` objects.
//            // Collect the streams into a single value.
//            // TODO: test what happens if the field value is way too long.
//            while let Some(chunk) = field.next().await {
//                bytes.extend_from_slice(&chunk.unwrap());
//            }
//            // Parse the bytes to a UTF-8 `String`.
//            // TODO: sanitize the string as well.
//            content = Some(std::str::from_utf8(&bytes).unwrap());
//            break;
//        }
//        handler::add_todo(&pool.get().await, content.unwrap().to_string()).await;
//        HttpResponse::SeeOther()
//            .insert_header((header::LOCATION, "/"))
//            .finish()
//    }
//
//    // Bind the TCP listener on the requested port.
//    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
//    let port = listener.local_addr().unwrap().port();
//    let server = HttpServer::new(move || {
//        App::new()
//            .app_data(Data::new(pool.clone()))
//            .service(index)
//            .service(add_todo)
//    })
//    .listen(listener)
//    .unwrap()
//    .run();
//
//    #[actix_web::main]
//    async fn serve_(server: impl Future<Output = Result<(), std::io::Error>>) {
//        server.await.unwrap()
//    }
//
//    (move || serve_(server), port)
//}
//
//mod handler {
//    use crate::Database;
//    use crate::Todo;
//    use crate::{get_100, insert};
//
//    /// Serializes a `GET /` response into a string slice.
//    pub async fn index(db: &Database) -> String {
//        // Fetch a list of all TODOs from the database.
//        let mut todos = get_100(db).await.peekable();
//
//        // Serialize the TODOs to HTML.
//        let todos = if todos.peek().is_some() {
//            let mut string = String::new();
//            string.push_str("<ul>");
//            for todo in todos {
//                string.push_str("<li>");
//                string.push_str(todo.content());
//                string.push_str("</li>");
//            }
//            string.push_str("</ul>");
//            string
//        } else {
//            String::new()
//        };
//
//        format!(
//            "<html>\
//            <head></head>\
//            <body>\
//            <form method=\"post\" action=\"/todo\" enctype=\"multipart/form-data\">\
//              <label for=\"content\">TODO:</label>\
//              <input type=\"text\" name=\"content\" id=\"content\" required>\
//              {}\
//            </form>\
//            </body>\
//            </html>",
//            todos
//        )
//    }
//
//    /// Serializes a `POST /todo` response into a string slice.
//    pub async fn add_todo(db: &Database, content: String) {
//        // Insert the TODO into the database.
//        insert(db, Todo(content)).await;
//    }
//}
