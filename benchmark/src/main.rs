use std::{
    env,
    fs::File,
    io::Write,
    str::FromStr,
    time::{Duration, Instant},
};

use route::{AlgorithmState, ExecutionType, Graph, PathResult};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        println!("Required: <Graph binary file> <Shortcut graph binary file> <execution type>");
        println!("Possible execution types:");
        for s in ExecutionType::get_strings() {
            println!("  - {}", s);
        }
        return;
    }

    let execute_all = &args[3].to_lowercase() == "all";

    let execution_type = if execute_all {
        ExecutionType::Dijkstra
    } else {
        match FromStr::from_str(&args[3]) {
            Ok(et) => et,
            Err(()) => {
                println!("Invalid execution type {}", &args[3]);
                return;
            }
        }
    };

    let graph = Graph::new_from_binfile(&args[1]);
    let shortcut_graph = if execute_all || execution_type.uses_shortcut() {
        Some(Graph::new_from_binfile(&args[2]))
    } else {
        None
    };
    let chosen_nodes = graph.generate_random_water_nodes(100);
    let mut state = AlgorithmState::new(graph.raster_columns_count * graph.raster_rows_count);

    if !execute_all {
        let g = if execution_type.uses_shortcut() {
            shortcut_graph.as_ref().unwrap()
        } else {
            &graph
        };
        println!("Measuring performance...");
        let (results, mut durations) =
            measure_performance(g, &mut state, &chosen_nodes, execution_type);

        println!("Validating results...");
        let mut correct_results = Vec::new();
        for (start_node, end_node) in chosen_nodes.iter() {
            correct_results.push(graph.dijkstra(*start_node, *end_node, &mut state));
        }

        // Print all routes in geojson format
        if false {
            let mut geojson = route::GEOJson {
                r#type: "FeatureCollection",
                features: Vec::new(),
            };
            for result in correct_results.iter() {
                if result.path.is_none() {
                    continue;
                }
                let mut coordinates = Vec::new();
                for node in result.path.as_ref().unwrap().iter() {
                    coordinates.push([graph.get_lon(*node), graph.get_lat(*node)]);
                }

                // Split up lines crossing the antimeridan
                let mut line_start = 0;
                let mut lon_start = 0.0;
                for i in 1..coordinates.len() {
                    if (coordinates[i - 1][0] - coordinates[i][0]).abs() > 180.0 {
                        let lon_end = if coordinates[i - 1][0] < 0.0 {
                            -180.0
                        } else {
                            180.0
                        };

                        let mut line_coordinates = Vec::new();
                        if line_start > 0 {
                            line_coordinates.push([lon_start, coordinates[line_start][1]]);
                        }
                        line_coordinates.extend_from_slice(&coordinates[line_start..i - 1]);
                        line_coordinates.push([lon_end, coordinates[i - 1][1]]);

                        geojson.features.push(route::GEOJsonFeature {
                            r#type: "Feature",
                            geometry: route::GEOJsonGeometry {
                                r#type: "LineString",
                                coordinates: line_coordinates,
                            },
                            properties: route::GEOJsonProperty {},
                        });

                        line_start = i;
                        lon_start = -lon_end;
                    }
                }

                let mut line_coordinates = Vec::new();
                if line_start > 0 {
                    line_coordinates.push([lon_start, coordinates[line_start][1]]);
                }
                line_coordinates.extend_from_slice(&coordinates[line_start..]);

                geojson.features.push(route::GEOJsonFeature {
                    r#type: "Feature",
                    geometry: route::GEOJsonGeometry {
                        r#type: "LineString",
                        coordinates: line_coordinates,
                    },
                    properties: route::GEOJsonProperty {},
                });
            }
            println!("{}", serde_json::to_string(&geojson).unwrap());
        }

        let mut differences = validate_results(&correct_results, &results, g, &chosen_nodes);

        print_statistics(&mut differences, &results, &chosen_nodes, &mut durations);

        let mut file = File::create("benchmark.txt").unwrap();
        write!(
            file,
            "{}",
            durations
                .iter()
                .map(|x| format!("{}", get_milliseconds(x)))
                .collect::<Vec<String>>()
                .join("\n")
        )
        .unwrap();
    } else {
        println!("Calculating correct results...");
        let mut correct_results = Vec::new();
        for (start_node, end_node) in chosen_nodes.iter() {
            correct_results.push(graph.dijkstra(*start_node, *end_node, &mut state));
        }

        let mut statistics = Vec::new();
        for algorithm in ExecutionType::get_strings() {
            println!("Measuring performance for {}", algorithm);
            let execution_type = ExecutionType::from_str(algorithm).unwrap();
            let g = if execution_type.uses_shortcut() {
                shortcut_graph.as_ref().unwrap()
            } else {
                &graph
            };
            let (results, durations) =
                measure_performance(g, &mut state, &chosen_nodes, execution_type);
            statistics.push((results, durations, Vec::<usize>::new()));
        }

        for (i, algorithm) in ExecutionType::get_strings().iter().enumerate() {
            println!("Validating {}", algorithm);
            let execution_type = ExecutionType::from_str(algorithm).unwrap();
            let g = if execution_type.uses_shortcut() {
                shortcut_graph.as_ref().unwrap()
            } else {
                &graph
            };
            let differences =
                validate_results(&correct_results, &statistics[i].0, g, &chosen_nodes);
            statistics[i].2 = differences;

            /* for (i, d) in statistics[i].1.iter().enumerate() {
                if d.as_millis() > 100 {
                    println!(
                        "{}ms for (N{},E{}) -> (N{},E{})",
                        get_milliseconds(d),
                        graph.get_lat(chosen_nodes[i].0),
                        graph.get_lon(chosen_nodes[i].0),
                        graph.get_lat(chosen_nodes[i].1),
                        graph.get_lon(chosen_nodes[i].1),
                    );
                }
            } */
        }

        for (i, (results, durations, differences)) in statistics.iter_mut().enumerate() {
            println!("\n{} statistics:", ExecutionType::get_strings()[i]);
            print_statistics(differences, results, &chosen_nodes, durations);
        }

        for (i, algorithm) in ExecutionType::get_strings().iter().enumerate() {
            let mut file = File::create(format!("benchmark{}.txt", algorithm)).unwrap();
            write!(
                file,
                "{}",
                statistics[i]
                    .1
                    .iter()
                    .map(|x| format!("{}", get_milliseconds(x)))
                    .collect::<Vec<String>>()
                    .join("\n")
            )
            .unwrap();
        }
    }
}

fn measure_performance(
    graph: &Graph,
    state: &mut AlgorithmState,
    chosen_nodes: &[(usize, usize)],
    execution_type: ExecutionType,
) -> (Vec<PathResult>, Vec<Duration>) {
    let mut results = Vec::new();
    let mut durations = Vec::new();
    for (start_node, end_node) in chosen_nodes.iter() {
        let start = Instant::now();

        let result = match execution_type {
            ExecutionType::Dijkstra => graph.dijkstra(*start_node, *end_node, state),
            ExecutionType::BiDijkstra => graph.bi_dijkstra(*start_node, *end_node, state),
            ExecutionType::AStar => graph.a_star(*start_node, *end_node, state),
            ExecutionType::ShortcutAStar => graph.shortcut_a_star(*start_node, *end_node, state),
            ExecutionType::ShortcutDijkstra => {
                graph.shortcut_dijkstra(*start_node, *end_node, state)
            }
        };

        let end = Instant::now();
        durations.push(end - start);
        results.push(result);
    }
    (results, durations)
}

fn validate_results(
    correct_results: &[PathResult],
    results: &[PathResult],
    graph: &Graph,
    chosen_nodes: &[(usize, usize)],
) -> Vec<usize> {
    let mut differences = Vec::new();
    for (i, (start_node, end_node)) in chosen_nodes.iter().enumerate() {
        let correct_result = &correct_results[i];
        let result = &results[i];
        assert_eq!(correct_result.distance.is_some(), result.distance.is_some());
        if correct_result.distance.is_some() {
            let d1 = correct_result.distance.unwrap();
            let d2 = result.distance.unwrap();
            let diff = if d1 > d2 { d1 - d2 } else { d2 - d1 };
            differences.push(diff as usize);

            if diff > 1000 {
                println!(
                    "High diff: {}km (N{},E{}) -> (N{},E{})",
                    diff as f64 / 1000.0,
                    graph.get_lat(*start_node),
                    graph.get_lon(*start_node),
                    graph.get_lat(*end_node),
                    graph.get_lon(*end_node)
                );
            }
        }
    }
    differences
}

fn print_statistics(
    differences: &mut Vec<usize>,
    results: &[PathResult],
    chosen_nodes: &[(usize, usize)],
    durations: &mut Vec<Duration>,
) {
    {
        differences.sort_unstable();
        let total = differences.iter().sum::<usize>() as f64 / 1000.0;
        let median = differences[differences.len() / 2] as f64 / 1000.0;
        let min = differences[0] as f64 / 1000.0;
        let max = differences[differences.len() - 1] as f64 / 1000.0;

        let width = (max as f64).log10().ceil() as usize + 3;
        println!(
            "\nAverage diff/node:   {:>1$.3}km",
            total as f64
                / results
                    .iter()
                    .map(|x| x.path.as_ref().unwrap_or(&Vec::new()).len())
                    .sum::<usize>() as f64,
            width
        );
        println!(
            "Average difference:  {:>1$.3}km",
            total / differences.len() as f64,
            width
        );
        println!("Median difference:   {:>1$.3}km", median, width);
        println!("Min difference:      {:>1$.3}km", min, width);
        println!("Max difference:      {:>1$.3}km", max, width);
    }
    {
        let amount = chosen_nodes.len();
        durations.sort();
        let mut total = 0.0;
        for duration in durations.iter() {
            total += get_milliseconds(duration);
        }
        let median = get_milliseconds(&durations[durations.len() / 2]);
        let min = get_milliseconds(&durations[0]);
        let max = get_milliseconds(&durations[durations.len() - 1]);

        let width = total.log10().ceil() as usize + 4;
        println!("\nStatistics for {} random queries:", amount);
        println!("Total:    {:>1$.3}ms", total, width);
        println!("Average:  {:>1$.3}ms", total / amount as f64, width);
        println!("Median:   {:>1$.3}ms", median, width);
        println!("Min:      {:>1$.3}ms", min, width);
        println!("Max:      {:>1$.3}ms", max, width);
    }
    {
        let mut heap_pops: Vec<usize> = results.iter().map(|x| x.heap_pops).collect();
        heap_pops.sort_unstable();
        let total: usize = heap_pops.iter().sum();
        let median = heap_pops[heap_pops.len() / 2];
        let min = heap_pops[0];
        let max = heap_pops[heap_pops.len() - 1];

        let width = (max as f64).log10().ceil() as usize;
        println!("\nHeap pops:");
        println!("Average:  {:>1$}", total / heap_pops.len(), width);
        println!("Median:   {:>1$}", median, width);
        println!("Min:      {:>1$}", min, width);
        println!("Max:      {:>1$}", max, width);
    }
}

fn get_milliseconds(duration: &Duration) -> f64 {
    duration.as_micros() as f64 / 1000.0
}
