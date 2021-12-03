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
                let input = rouille::try_or_400!(rouille::post_input!(request, {
                    lat1: f64,
                    lon1: f64,
                    lat2: f64,
                    lon2: f64,
                }));

                println!("Marker 1 at: {},{}", input.lon1, input.lat1);
                println!("Marker 2 at: {},{}", input.lon2, input.lat2);

                let distance = Graph::calculate_distance(input.lon1,input.lat1,input.lon2,input.lat2);
                println!("Distance: {} km", distance);

                let geojson = graph.find_path(input.lon1, input.lat1, input.lon2, input.lat2);

                Response::text(format!("Some POST response, body was: {}", ""))
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
