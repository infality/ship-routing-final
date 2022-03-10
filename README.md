## Extraction

Use `cargo run --release --bin extract <pbf file>` to create a `graph.bin` file which contains a graph with one million nodes and their connected edges between water nodes.

## Routing

Run `cargo run --release --bin route -- <graph file> <algorithm>` to host a local webserver which can be access under `http://localhost:8000/`.

As graph file you can either choose the normal graph that is computed in the extraction step or a shortcut graph that was generated during shortcut creation.
For the algorithm you can choose between following:
* Dijkstra
* BiDijkstra
* AStar
* ShortcutAStar
* ShortcutDijkstra

## Shortcut creation

Use `cargo run --release --bin create_shortcuts -- --select <graph file>` to start the shortcut rectangle selection. This hosts a local webserver which can be access under `http://localhost:8000/`. There you can click on a spot in the water to create a rectangle at that location which expands until land is reached or the rectangle sides exceed 50 nodes. To delete a rectangle click it again. During this process the coordinates of all chosen rectangles are printed on the console.

To extend a graph file with the just selected rectangles you have to run `cargo run --release --bin create_shortcuts -- --create <graph file> "<rectangle coordinates>"` where `<rectangle coordinates>` are the coordinates printed onto the console during the selection proccess. This creates a new graph file called `graph_shortcuts.bin` that contains all original edges from the graph file along with the new edges required to shortcut the rectangle areas.

## Benchmark

To benchmark a single or all algorithms for 100 random queries use `cargo run --release --bin benchmark -- <graph file> <shortcut graph file> <algorithm>`. The program automatically chooses the shortcut graph for algorithms requiring shortcuts.
For the algorithm you can choose between following:
* Dijkstra
* BiDijkstra
* AStar
* ShortcutAStar
* ShortcutDijkstra
* All
If `All` is chosen all algorithms are tested consecutively using the same 100 queries.

After benchmarking one or multiple text files are created which can be copied to the `benchmarks` directory and then visualized using the gnuplot script with `gnuplot -p <path to boxplot.gnu>`.

