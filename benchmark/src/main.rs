use std::{
    env,
    fs::File,
    io::Write,
    time::{Duration, Instant},
};

use route::Graph;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Please pass a graph binary file");
        return;
    }

    let graph = Graph::new_from_binfile(&args[1]);
    let chosen_nodes = graph.generate_random_water_nodes(1000);

    let mut durations = Vec::new();
    for (start_node, end_node) in chosen_nodes.iter() {
        let start = Instant::now();
        graph.dijkstra(*start_node, *end_node);
        let end = Instant::now();
        durations.push(end - start);
    }

    let amount = chosen_nodes.len();
    let mut total = 0.0;
    durations.sort();
    for duration in durations.iter() {
        total += get_milliseconds(duration);
    }
    let median = get_milliseconds(&durations[durations.len() / 2]);
    let min = get_milliseconds(&durations[0]);
    let max = get_milliseconds(&durations[durations.len() - 1]);

    let width = total.log10().ceil() as usize + 4;
    println!("Statistics for {} random queries:", amount);
    println!("Total:    {:>1$.3}ms", total, width);
    println!("Average:  {:>1$.3}ms", total / amount as f64, width);
    println!("Median:   {:>1$.3}ms", median, width);
    println!("Min:      {:>1$.3}ms", min, width);
    println!("Max:      {:>1$.3}ms", max, width);

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
