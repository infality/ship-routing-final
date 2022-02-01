use rouille::Response;
use std::env;

use route::{GEOJson, GEOJsonFeature, GEOJsonGeometry, GEOJsonProperty, Graph};

#[derive(serde::Serialize)]
struct NodePositions {
    geojson: GEOJson<Vec<f64>>,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Please pass a graph binary file");
        return;
    }

    let html_file = include_str!("index.html");

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

                println!("Clicked at: {},{}", input.lon1, input.lat1);

                let mut geojson = GEOJson {
                    r#type: "FeatureCollection",
                    features: Vec::new(),
                };

                let node_positions = NodePositions { geojson };

                return Response::json(&node_positions);
            },

            (GET) (/allPoints) => {
                let mut geojson = GEOJson {
                    r#type: "FeatureCollection",
                    features: Vec::new(),
                };

                for i in 0..(graph.raster_rows_count * graph.raster_colums_count) {
                    if graph.offsets[i] != graph.offsets[i + 1] {
                        geojson.features.push(GEOJsonFeature {
                            r#type: "Feature",
                            properties: GEOJsonProperty {},
                            geometry: GEOJsonGeometry {
                                r#type: "Point",
                                coordinates: vec![graph.get_lon(i), graph.get_lat(i)],
                            },
                        });
                    }
                }

                let node_positions = NodePositions { geojson };

                return Response::json(&node_positions);
            },

            _ => Response::empty_404(),
        )
    });
}
