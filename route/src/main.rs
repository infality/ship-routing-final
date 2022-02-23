use rouille::Response;
use std::{env, str::FromStr};

mod lib;
use lib::{AlgorithmState, ExecutionType, GEOJson, Graph};

#[derive(serde::Serialize)]
struct RouteResponse {
    geojson: GEOJson<Vec<[f64; 2]>>,
    //geojson: GEOJson<[f64; 2]>,
    distance: f64,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Required: <Graph binary file> <execution type>");
        println!("Possible execution types:");
        for s in ExecutionType::get_strings() {
            println!("  - {}", s);
        }
        return;
    }

    let execution_type = match FromStr::from_str(&args[2]) {
        Ok(et) => et,
        Err(()) => {
            println!("Invalid execution type {}", &args[2]);
            return;
        }
    };

    let html_file = include_str!("index.html");
    let marker_icon = include_bytes!("marker-icon.png");
    let marker_icon2 = include_bytes!("marker-icon2.png");

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

                let mut state = AlgorithmState::new(graph.raster_columns_count * graph.raster_rows_count);
                let result = graph.find_path(input.lon1, input.lat1, input.lon2, input.lat2, &execution_type, &mut state);
                println!("Done!\n");
                if let Some((geojson, distance)) = result {
                    let route_response = RouteResponse {geojson, distance};
                    return Response::json(&route_response);
                }
                Response::text("{}")
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
