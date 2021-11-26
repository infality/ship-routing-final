use rouille::Response;
use std::env;
use std::io::Read;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Please pass a graph binary file");
        return;
    }

    // TODO Load graph and setup routing

    rouille::start_server("127.0.0.1:8000", move |request| match request.method() {
        "GET" => {
            // TODO Return html file
            return Response::text("Some GET response");
        }
        "POST" => {
            let request_body = request.data();
            let mut content = String::new();
            if let Some(mut body) = request_body {
                let result = body.read_to_string(&mut content);
                if result.is_err() {
                    println!("Error reading POST body: {}", result.unwrap_err());
                }
            }

            // TODO Parse coordinates and return geojson of route

            return Response::text(format!("Some POST response, body was: {}", content));
        }
        _ => Response::empty_404(),
    });
}
