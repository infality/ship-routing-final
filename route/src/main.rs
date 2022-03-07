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

/* #[derive(serde::Serialize, serde::Deserialize)]
pub struct Graph2 {
    pub offsets: Vec<u32>,
    pub edges: Vec<lib::Edge>,
    pub raster_columns_count: usize,
    pub raster_rows_count: usize,
    pub shortcut_rectangles: Vec<(usize, usize, usize, usize)>,
} */

fn main() {
    /* let mut buf_reader = std::io::BufReader::new(std::fs::File::open("graph_shortcuts_old.bin").unwrap());
    let graph: Graph2 = bincode::deserialize_from(&mut buf_reader).unwrap();

    let mut graph2 = Graph {
        offsets: Vec::new(),
        edges: graph.edges.clone(),
        raster_columns_count: graph.raster_columns_count,
        raster_rows_count: graph.raster_rows_count,
        shortcut_rectangles: graph.shortcut_rectangles.clone(),
    };
    for (i, offset) in graph.offsets.iter().enumerate() {
        let mut is_in_rect = None;
        if i < graph2.raster_rows_count * graph2.raster_columns_count {
            for (rect_index, rect) in graph2.shortcut_rectangles.iter().enumerate() {
                if graph2.is_node_inside_rect(i, rect) {
                    is_in_rect = Some(rect_index);
                    break;
                }
            }
        }
        graph2.offsets.push((*offset, is_in_rect));
    }

    graph2.write_to_binfile("graph_shortcuts.bin");

    return; */

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
