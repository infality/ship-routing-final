use std::{
    env,
    fs::File,
    io::Write,
    time::{Duration, Instant},
};

use route::{AlgorithmState, Graph};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Please pass a graph binary file");
        return;
    }

    let graph = Graph::new_from_binfile(&args[1]);
    let chosen_nodes = graph.generate_random_water_nodes(100);
    let mut results = Vec::new();
    let mut state = AlgorithmState::new(graph.raster_colums_count * graph.raster_rows_count);

    // Measure algorithm performance
    println!("Measuring performance...");
    let mut durations = Vec::new();
    for (start_node, end_node) in chosen_nodes.iter() {
        let start = Instant::now();
        let result = graph.a_star(*start_node, *end_node, &mut state);
        let end = Instant::now();
        durations.push(end - start);
        results.push(result);
    }

    // Validate results with dijkstra
    println!("Validating results...");
    let mut differences = Vec::new();
    for (i, (start_node, end_node)) in chosen_nodes.iter().enumerate() {
        let result = graph.dijkstra(*start_node, *end_node, &mut state);
        assert_eq!(result.distance.is_some(), results[i].distance.is_some());
        if result.distance.is_some() {
            let d1 = result.distance.unwrap();
            let d2 = results[i].distance.unwrap();
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
}

fn get_milliseconds(duration: &Duration) -> f64 {
    duration.as_micros() as f64 / 1000.0
}
