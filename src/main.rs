use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::fs::File;
use std::io::Read;

const CHUNK_SIZE: usize = 1024 * 1024 * 2; // Chunk size in bytes
const RATE_LIMIT_DELAY_MS: u64 = 1; // Rate limit delay in milliseconds
const MAX_CONCURRENT_REQUESTS: usize = 10; // Max concurrent requests

async fn serve_video(
    _req: Request<Body>,
    video_name: &str,
) -> Result<Response<Body>, hyper::Error> {
    // Read video file from disk

    // Find the index of '='
    let mut v = "";
    if let Some(index) = video_name.find('=') {
        // Get the substring after '='
        v = &video_name[(index + 1)..];
        println!("{}", v); // Print the substring after '=' which is "xxyy"
    } else {
        println!("No '=' found");
    }
    let video_name = v;
    println!("video:{}", video_name);
    let mut file = match File::open(format!("videos/{}", video_name)) {
        Ok(file) => file,
        Err(_) => {
            println!("not found");
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap());
        }
    };
    let (mut sender, body) = Body::channel();
    println!("body: {:?}", body);
    // Create a buffer for chunking
    let mut buffer = vec![0; CHUNK_SIZE];
    let mut total_bytes_sent: usize = 0;
    tokio::spawn(async move {
        loop {
            match file.read(&mut buffer) {
                Ok(0) => {
                    println!("eof");
                    break;
                } // End of file
                Ok(n) => {
                    total_bytes_sent += n;
                    sender
                        .send_data(hyper::body::Bytes::from(buffer[..n].to_vec()))
                        .await
                        .expect("connection closed");
                }
                Err(_) => {
                    println!("crashed bruv");
                }
            }
            println!("sending {}", total_bytes_sent);
        }
        drop(sender);
    });
    println!("done dawg");
    Ok(Response::new(body))
}
async fn serve_file(path: &str) -> Result<Response<Body>, hyper::Error> {
    // Read the file content
    let mut file = File::open(path).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();

    // Create a response with the file content
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(buf))
        .unwrap())
}
async fn handle_request(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        // Handle GET requests
        (&hyper::Method::GET, "/video") => {
            // Extract query parameters
            let uri = req.uri().clone();
            let query_params = uri.path_and_query().expect("ok");

            if let Some(video_name) = query_params.query() {
                serve_video(req, video_name).await
            } else {
                let response = Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Missing video_name parameter"))
                    .unwrap();
                Ok(response)
            }
        }
        (&hyper::Method::GET, "/home") => serve_file("html/home.html").await,
        // Handle other requests
        _ => {
            let response = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("<p>well .. ....<p>"))
                .unwrap();
            Ok(response)
        }
    }
}

#[tokio::main]
async fn main() {
    let addr = ([127, 0, 0, 1], 4000).into();

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Server running at http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
