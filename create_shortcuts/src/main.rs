use rayon::prelude::*;
use rouille::Response;
use std::{env, sync::Mutex};

use route::{
    AlgorithmState, Edge, GEOJson, GEOJsonFeature, GEOJsonGeometry, GEOJsonProperty, Graph,
};

#[derive(serde::Serialize)]
struct ShortcutRectangle {
    geojson: GEOJson<[Vec<[f64; 2]>; 1]>,
}

fn get_index(graph: &Graph, col: usize, row: usize) -> usize {
    row * graph.raster_columns_count + col
}

fn is_water(graph: &Graph, col: usize, row: usize) -> bool {
    let index = row * graph.raster_columns_count + col;
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
    for (i, (left, top, right, bottom)) in rects.iter().enumerate() {
        print!("{},{},{},{}", left, top, right, bottom);
        if i < rects.len() - 1 {
            print!(";");
        }
    }
    println!("\n");

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
                        graph.get_lat(top * graph.raster_columns_count),
                    ],
                    [
                        graph.get_lon(*right),
                        graph.get_lat(top * graph.raster_columns_count),
                    ],
                    [
                        graph.get_lon(*right),
                        graph.get_lat(bottom * graph.raster_columns_count),
                    ],
                    [
                        graph.get_lon(*left),
                        graph.get_lat(bottom * graph.raster_columns_count),
                    ],
                    [
                        graph.get_lon(*left),
                        graph.get_lat(top * graph.raster_columns_count),
                    ],
                ]],
            },
        });
    }
    ShortcutRectangle { geojson }
}

// Add edges for both directions
fn add_edges(
    graph: &Graph,
    edges: &mut [Vec<Edge>],
    index1: usize,
    index2: usize,
    state: &mut AlgorithmState,
) {
    let distance = graph.bi_dijkstra(index1, index2, state).distance.unwrap();
    edges[index1].push(Edge {
        destination: index2 as u32,
        distance,
    });
    edges[index2].push(Edge {
        destination: index1 as u32,
        distance,
    });
}

fn create_graph(graph: &Graph, rects: &[(usize, usize, usize, usize)]) -> Graph {
    let node_count = graph.raster_rows_count * graph.raster_columns_count;
    let edges = Mutex::new(vec![Vec::<Edge>::new(); node_count]);

    // Add original graph edges
    for (i, edge) in edges.lock().unwrap().iter_mut().enumerate() {
        for e in graph.offsets[i]..graph.offsets[i + 1] {
            edge.push(graph.edges[e as usize]);
        }
    }

    // Use multiple threads to add all possible shortcut edges for each rectangle
    rects
        .par_iter()
        .enumerate()
        .for_each(|(i, (left, top, right, bottom))| {
            let left_count = bottom - top;
            let top_count = right - left;
            println!(
                "Starting thread to add edges for rectangle {}/{} ({} distance calculations)",
                i + 1,
                rects.len(),
                4 * left_count * top_count + left_count * left_count + top_count * top_count
            );

            let mut state = AlgorithmState::new(node_count);
            let mut local_edges = vec![Vec::<Edge>::new(); node_count];
            for l in *top..=*bottom {
                let li = get_index(graph, *left, l);

                for t in *left..=*right {
                    let ti = get_index(graph, t, *top);
                    add_edges(graph, &mut local_edges, li, ti, &mut state);
                }

                for r in *top..=*bottom {
                    let ri = get_index(graph, *right, r);
                    add_edges(graph, &mut local_edges, li, ri, &mut state);
                }

                for b in *left..=*right {
                    let bi = get_index(graph, b, *bottom);
                    add_edges(graph, &mut local_edges, li, bi, &mut state);
                }
            }

            for t in *left..=*right {
                let ti = get_index(graph, t, *top);

                for r in *top..=*bottom {
                    let ri = get_index(graph, *right, r);
                    add_edges(graph, &mut local_edges, ti, ri, &mut state);
                }

                for b in *left..=*right {
                    let bi = get_index(graph, b, *bottom);
                    add_edges(graph, &mut local_edges, ti, bi, &mut state);
                }
            }

            for r in *top..=*bottom {
                let ri = get_index(graph, *right, r);

                for b in *left..=*right {
                    let bi = get_index(graph, b, *bottom);
                    add_edges(graph, &mut local_edges, ri, bi, &mut state);
                }
            }

            let mut edges_lock = edges.lock().unwrap();
            for (i, node_edges) in local_edges.iter().enumerate() {
                for dest in node_edges.iter() {
                    edges_lock[i].push(*dest);
                }
            }
            println!("Thread for rectangle {} finished", i + 1);
        });

    let mut new_graph = Graph {
        offsets: Vec::with_capacity(node_count + 1),
        edges: Vec::new(),
        raster_columns_count: graph.raster_columns_count,
        raster_rows_count: graph.raster_rows_count,
        shortcut_rectangles: rects.to_vec(),
    };

    for edge in edges.lock().unwrap().iter() {
        new_graph.offsets.push(new_graph.edges.len() as u32);
        for edge in edge.iter() {
            new_graph.edges.push(*edge);
        }
    }
    new_graph.offsets.push(new_graph.edges.len() as u32);

    new_graph
}

#[allow(unreachable_code)]
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Options:");
        println!("  --select <graph file>");
        println!("  --create <graph file> <shortcut rectangles>");
        println!("\nTo either select shortcut rectangles or to create a new graph file with passed shortcut rectangles string (retrieved during selection)");
        return;
    }
    let graph = Graph::new_from_binfile(&args[2]);

    if args[1] == "--create" {
        let mut rects = Vec::new();
        for rect in args[3].split(';') {
            let sides: Vec<&str> = rect.splitn(4, ',').collect();
            let left = sides[0].parse().unwrap();
            let top = sides[1].parse().unwrap();
            let right = sides[2].parse().unwrap();
            let bottom = sides[3].parse().unwrap();
            rects.push((left, top, right, bottom));
        }
        let new_graph = create_graph(&graph, &rects);
        new_graph.write_to_binfile("graph_shortcuts.bin");
        return;
    }
    if args[1] != "--select" {
        println!("Unknown option");
        return;
    }

    let html_file = include_str!("index.html");
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
                let mut left = clicked_pos % graph.raster_columns_count;
                let mut right = left;
                let mut top = clicked_pos / graph.raster_columns_count;
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
                            if right == graph.raster_columns_count - 1 {
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
