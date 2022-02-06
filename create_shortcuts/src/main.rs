use rouille::Response;
use std::{env, sync::Mutex};

use route::{GEOJson, GEOJsonFeature, GEOJsonGeometry, GEOJsonProperty, Graph};

#[derive(serde::Serialize)]
struct ShortcutRectangle {
    geojson: GEOJson<[Vec<[f64; 2]>; 1]>,
}

fn is_water(graph: &Graph, col: usize, row: usize) -> bool {
    let index = row * graph.raster_colums_count + col;
    graph.offsets[index] != graph.offsets[index + 1]
}

fn find_colliding_rect(
    rects: &[(usize, usize, usize, usize)],
    (left, top, right, bottom): (usize, usize, usize, usize),
) -> Option<usize> {
    for (i, (rleft, rtop, rright, rbottom)) in rects.iter().enumerate() {
        if left < *rright && right > *rleft && top < *rbottom && bottom > *rtop {
            return Some(i);
        }
    }
    None
}

fn create_geojson(graph: &Graph, rects: &[(usize, usize, usize, usize)]) -> ShortcutRectangle {
    println!("\nRectangles:");
    for (left, top, right, bottom) in rects.iter() {
        println!("{},{},{},{}", left, top, right, bottom);
    }
    println!();

    let mut geojson = GEOJson {
        r#type: "FeatureCollection",
        features: Vec::new(),
    };

    for (left, top, right, bottom) in rects.iter() {
        geojson.features.push(GEOJsonFeature {
            r#type: "Feature",
            properties: GEOJsonProperty {},
            geometry: GEOJsonGeometry {
                r#type: "Polygon",
                coordinates: [vec![
                    [
                        graph.get_lon(*left),
                        graph.get_lat(top * graph.raster_colums_count),
                    ],
                    [
                        graph.get_lon(*right),
                        graph.get_lat(top * graph.raster_colums_count),
                    ],
                    [
                        graph.get_lon(*right),
                        graph.get_lat(bottom * graph.raster_colums_count),
                    ],
                    [
                        graph.get_lon(*left),
                        graph.get_lat(bottom * graph.raster_colums_count),
                    ],
                    [
                        graph.get_lon(*left),
                        graph.get_lat(top * graph.raster_colums_count),
                    ],
                ]],
            },
        });
    }
    ShortcutRectangle { geojson }
}

#[allow(unreachable_code)]
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Please pass a graph binary file");
        return;
    }

    let html_file = include_str!("index.html");

    let graph = Graph::new_from_binfile(&args[1]);
    let placed_rectangles = Mutex::new(Vec::<(usize, usize, usize, usize)>::new()); // left, top, right, bottom

    rouille::start_server("localhost:8000", move |request| {
        rouille::router!(request,
            (GET) ["/"] => {
                placed_rectangles.lock().unwrap().clear();
                rouille::Response::html(html_file)
            },

            (POST) ["/"] => {
                let mut placed_rectangles = placed_rectangles.lock().unwrap();
                let input = rouille::try_or_400!(rouille::post_input!(request, {
                    lat: f64,
                    lon: f64,
                }));

                println!("Clicked at: {},{}", input.lon, input.lat);

                let clicked_pos = graph.find_nearest_node(input.lon, input.lat);
                if clicked_pos.is_none() {
                    return Response::json(&create_geojson(&graph, &placed_rectangles));
                }
                let clicked_pos = clicked_pos.unwrap();

                // Row and columns of expanding rectangle
                let mut left = clicked_pos % graph.raster_colums_count;
                let mut right = left;
                let mut top = clicked_pos / graph.raster_colums_count;
                let mut bottom = top;

                if let Some(rect) = find_colliding_rect(&placed_rectangles, (left,top,right,bottom)) {
                    placed_rectangles.remove(rect);
                    return Response::json(&create_geojson(&graph, &placed_rectangles));
                }

                let mut is_left_done = false;
                let mut is_right_done = false;
                let mut is_top_done = false;
                let mut is_bottom_done = false;
                while !is_left_done || !is_top_done || !is_right_done || !is_bottom_done {
                    if !is_left_done {
                        for row in top..=bottom {
                            if !is_water(&graph, left - 1, row) {
                                is_left_done = true;
                                break;
                            }
                            if find_colliding_rect(&placed_rectangles, (left - 1, top, right, bottom)).is_some() {
                                is_left_done = true;
                                break;
                            }
                        }
                        if !is_left_done {
                            left -= 1;
                            if left == 0 {
                                is_left_done = true;
                            }
                        }
                    }
                    if !is_top_done {
                        for col in left..=right {
                            if !is_water(&graph, col, top - 1) {
                                is_top_done = true;
                                break;
                            }
                            if find_colliding_rect(&placed_rectangles, (left, top - 1, right, bottom)).is_some() {
                                is_top_done = true;
                                break;
                            }
                        }
                        if !is_top_done {
                            top -= 1;
                            if top == 0 {
                                is_top_done = true;
                            }
                        }
                    }
                    if !is_right_done {
                        for row in top..=bottom {
                            if !is_water(&graph, right + 1, row) {
                                is_right_done = true;
                                break;
                            }
                            if find_colliding_rect(&placed_rectangles, (left, top, right + 1, bottom)).is_some() {
                                is_right_done = true;
                                break;
                            }
                        }
                        if !is_right_done {
                            right += 1;
                            if right == graph.raster_colums_count - 1 {
                                is_right_done = true;
                            }
                        }
                    }
                    if !is_bottom_done {
                        for col in left..=right {
                            if !is_water(&graph, col, bottom + 1) {
                                is_bottom_done = true;
                                break;
                            }
                            if find_colliding_rect(&placed_rectangles, (left, top, right, bottom + 1)).is_some() {
                                is_bottom_done = true;
                                break;
                            }
                        }
                        if !is_bottom_done {
                            bottom += 1;
                            if bottom == graph.raster_rows_count - 1 {
                                is_bottom_done = true;
                            }
                        }
                    }
                }

                if left == right || top == bottom {
                    return Response::json(&create_geojson(&graph, &placed_rectangles));
                }

                placed_rectangles.push((left, top, right, bottom));

                Response::json(&create_geojson(&graph, &placed_rectangles))
            },

            _ => Response::empty_404(),
        )
    });
}