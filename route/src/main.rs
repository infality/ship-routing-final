use rouille::Response;
use std::env;
use std::io::Read;

mod lib;
use lib::Graph;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Please pass a graph binary file");
        return;
    }

    let html_file = include_str!("index.html");
    let marker_icon = include_bytes!("marker-icon.png");
    let marker_icon2 = include_bytes!("marker-icon2.png");

    // TODO Load graph and setup routing
    let graph = Graph::new_from_binfile(&args[1]);

    rouille::start_server("localhost:8000", move |request| {
        rouille::router!(request,
            (GET) (/) => {
                rouille::Response::html(html_file)
            },

            (POST) (/) => {
                let request_body = request.data();
                let mut content = String::new();
                if let Some(mut body) = request_body {
                    let result = body.read_to_string(&mut content);
                    if result.is_err() {
                        println!("Error reading POST body: {}", result.unwrap_err());
                    }
                }

                // TODO Parse coordinates and return geojson of route

                Response::text(format!("Some POST response, body was: {}", content))
            },

            (GET) (/marker-icon) => {
                rouille::Response::from_data("image/png", marker_icon.to_vec())
            },

            (GET) (/marker-icon2) => {
                rouille::Response::from_data("image/png", marker_icon2.to_vec())
            },

            _ => Response::empty_404(),
        )
    });
}
