use std::str::FromStr;
use std::time::Instant;
use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    fs::File,
    io::{BufReader, BufWriter},
};

use rand::Rng;

const FACTOR: f64 = 10_000_000.0;

pub enum ExecutionType {
    Dijkstra,
    BiDijkstra,
    AStar,
    ShortcutAStar,
}

impl ExecutionType {
    pub fn get_strings() -> Vec<&'static str> {
        vec!["Dijkstra", "BiDijkstra", "AStar", "ShortcutAStar"]
    }

    pub fn uses_shortcut(&self) -> bool {
        matches!(self, ExecutionType::ShortcutAStar)
    }
}

impl FromStr for ExecutionType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dijkstra" => Ok(ExecutionType::Dijkstra),
            "bidijkstra" => Ok(ExecutionType::BiDijkstra),
            "astar" => Ok(ExecutionType::AStar),
            "shortcutastar" => Ok(ExecutionType::ShortcutAStar),
            _ => Err(()),
        }
    }
}

#[derive(Eq, PartialEq)]
pub struct HeapNode {
    pub id: u32,
    pub distance: u32,
}

impl Ord for HeapNode {
    fn cmp(&self, other: &HeapNode) -> Ordering {
        other
            .distance
            .cmp(&self.distance)
            .then_with(|| self.id.cmp(&other.id))
    }
}

impl PartialOrd for HeapNode {
    fn partial_cmp(&self, other: &HeapNode) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Graph starts at top left, outer arrays are rows
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Graph {
    pub offsets: Vec<(u32, Option<usize>)>, // Holds offset to edges and a bool determining if the node is inside a shortcut rectangle
    pub edges: Vec<Edge>,
    pub raster_columns_count: usize,
    pub raster_rows_count: usize,
    pub shortcut_rectangles: Vec<(usize, usize, usize, usize)>,
}

#[derive(serde::Serialize, serde::Deserialize, Copy, Clone)]
pub struct Edge {
    pub destination: u32,
    pub distance: u32,
}

pub struct PathResult {
    pub path: Option<Vec<usize>>,
    pub distance: Option<u32>,
    pub heap_pops: usize,
}

pub struct AlgorithmState {
    pub distances: Vec<u32>,
    pub parent_nodes: Vec<u32>,
    pub queue: BinaryHeap<HeapNode>,

    // Used for bidirectional algorithms
    pub distances2: Vec<u32>,
    pub parent_nodes2: Vec<u32>,
    pub queue2: BinaryHeap<HeapNode>,
}

impl AlgorithmState {
    pub fn new(node_count: usize) -> Self {
        AlgorithmState {
            distances: vec![std::u32::MAX; node_count],
            parent_nodes: vec![std::u32::MAX; node_count],
            queue: BinaryHeap::with_capacity(node_count),

            distances2: vec![std::u32::MAX; node_count],
            parent_nodes2: vec![std::u32::MAX; node_count],
            queue2: BinaryHeap::with_capacity(node_count),
        }
    }

    pub fn reset(&mut self) {
        for i in 0..self.distances.len() {
            self.distances[i] = std::u32::MAX;
            self.parent_nodes[i] = std::u32::MAX;
            self.distances2[i] = std::u32::MAX;
            self.parent_nodes2[i] = std::u32::MAX;
        }
        self.queue.clear();
        self.queue2.clear();
    }
}

#[derive(serde::Serialize)]
pub struct GEOJson<T> {
    pub r#type: &'static str,
    pub features: Vec<GEOJsonFeature<T>>,
}

#[derive(serde::Serialize)]
pub struct GEOJsonFeature<T> {
    pub r#type: &'static str,
    pub geometry: GEOJsonGeometry<T>,
    pub properties: GEOJsonProperty,
}

#[derive(serde::Serialize)]
pub struct GEOJsonGeometry<T> {
    pub r#type: &'static str,
    pub coordinates: T,
}

#[derive(serde::Serialize)]
pub struct GEOJsonProperty {}

impl Graph {
    pub fn is_node_inside_rect(&self, node: usize, rect: &(usize, usize, usize, usize)) -> bool {
        rect.0 < node % self.raster_columns_count
            && rect.1 < node / self.raster_columns_count
            && node % self.raster_columns_count < rect.2
            && node / self.raster_columns_count < rect.3
    }

    pub fn find_path(
        &self,
        lon1: f64,
        lat1: f64,
        lon2: f64,
        lat2: f64,
        execution_type: &ExecutionType,
        state: &mut AlgorithmState,
    ) -> Option<(GEOJson<Vec<[f64; 2]>>, f64)> {
        //) -> Option<(GEOJson<[f64; 2]>, f64)> {
        let mut now = Instant::now();
        let nearest_start_node = self.find_nearest_node(lon1, lat1);
        let nearest_end_node = self.find_nearest_node(lon2, lat2);
        println!(
            "Time taken for nearest node search: {}ms",
            now.elapsed().as_micros() as f32 / 1000.
        );
        now = Instant::now();

        let mut coordinates = vec![[lon2, lat2]];
        let mut distance = 0;

        if nearest_start_node.is_none() || nearest_end_node.is_none() {
            println!("No nearest start or end node found");
            return None;
        }

        let nearest_start_node = nearest_start_node.unwrap();
        let nearest_end_node = nearest_end_node.unwrap();

        println!(
            "Nearest start node: {},{}",
            self.get_lon(nearest_start_node),
            self.get_lat(nearest_start_node)
        );
        println!(
            "Nearest end node: {},{}",
            self.get_lon(nearest_end_node),
            self.get_lat(nearest_end_node)
        );

        if nearest_start_node == nearest_end_node {
            println!("Start node is equal to end node. Skipping search algorithm");
            distance += Self::calculate_distance(lon1, lat1, lon2, lat2);
        } else {
            println!("Start node is not equal to end node. Executing search algorithm");
            let result = match execution_type {
                ExecutionType::Dijkstra => {
                    self.dijkstra(nearest_start_node, nearest_end_node, state)
                }
                ExecutionType::BiDijkstra => {
                    self.bi_dijkstra(nearest_start_node, nearest_end_node, state)
                }
                ExecutionType::AStar => self.a_star(nearest_start_node, nearest_end_node, state),
                ExecutionType::ShortcutAStar => {
                    self.shortcut_a_star(nearest_start_node, nearest_end_node, state)
                }
            };

            if result.path.is_none() || result.distance.is_none() {
                println!(
                    "Search algorithm did not find a route and took {}ms",
                    now.elapsed().as_micros() as f32 / 1000.
                );
                return None;
            } else {
                println!(
                    "Search algorithm found a route and took {}ms",
                    now.elapsed().as_micros() as f32 / 1000.
                );
                let path = result.path.unwrap();
                distance += result.distance.unwrap();
                println!("Path length: {}", path.len());

                for node in path.iter() {
                    coordinates.push([self.get_lon(*node), self.get_lat(*node)]);
                }

                distance += Self::calculate_distance(
                    lon1,
                    lat1,
                    self.get_lon(*path.last().unwrap()),
                    self.get_lat(*path.last().unwrap()),
                );
                distance += Self::calculate_distance(
                    self.get_lon(path[0]),
                    self.get_lat(path[0]),
                    lon2,
                    lat2,
                );
            }
        }

        coordinates.push([lon1, lat1]);

        let mut geojson = GEOJson {
            r#type: "FeatureCollection",
            features: Vec::new(),
        };

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

                geojson.features.push(GEOJsonFeature {
                    r#type: "Feature",
                    geometry: GEOJsonGeometry {
                        r#type: "LineString",
                        coordinates: line_coordinates,
                    },
                    properties: GEOJsonProperty {},
                });

                /* for c in line_coordinates.iter() {
                    geojson.features.push(GEOJsonFeature {
                        r#type: "Feature",
                        geometry: GEOJsonGeometry {
                            r#type: "Point",
                            coordinates: *c,
                        },
                        properties: GEOJsonProperty {},
                    });
                } */

                line_start = i;
                lon_start = -lon_end;
            }
        }

        let mut line_coordinates = Vec::new();
        if line_start > 0 {
            line_coordinates.push([lon_start, coordinates[line_start][1]]);
        }
        line_coordinates.extend_from_slice(&coordinates[line_start..]);

        geojson.features.push(GEOJsonFeature {
            r#type: "Feature",
            geometry: GEOJsonGeometry {
                r#type: "LineString",
                coordinates: line_coordinates,
            },
            properties: GEOJsonProperty {},
        });

        /* for c in line_coordinates.iter() {
            geojson.features.push(GEOJsonFeature {
                r#type: "Feature",
                geometry: GEOJsonGeometry {
                    r#type: "Point",
                    coordinates: *c,
                },
                properties: GEOJsonProperty {},
            });
        } */

        Some((geojson, distance as f64))
    }

    pub fn find_nearest_node(&self, lon: f64, lat: f64) -> Option<usize> {
        let step_size_lon = (360_0000000.0 / self.raster_columns_count as f64) as usize;
        let lon_index_left = ((lon + 180.) * FACTOR) as usize / step_size_lon;
        let lon_index_right = (lon_index_left + 1) % self.raster_columns_count;

        let step_size_lat = (180_0000000.0 / self.raster_rows_count as f64) as usize;
        let lat_index_top = ((-lat + 90.) * FACTOR) as usize / step_size_lat;
        let lat_index_bottom = (lat_index_top + 1) % self.raster_rows_count;

        let neighbor_ids = vec![
            (lat_index_top * self.raster_columns_count) + lon_index_left,
            (lat_index_top * self.raster_columns_count) + lon_index_right,
            (lat_index_bottom * self.raster_columns_count) + lon_index_left,
            (lat_index_bottom * self.raster_columns_count) + lon_index_right,
        ];

        let mut best_neighbor = neighbor_ids[0];
        let mut min_distance = u32::MAX;
        for neighbor in neighbor_ids {
            let distance =
                Self::calculate_distance(lon, lat, self.get_lon(neighbor), self.get_lat(neighbor));

            let is_neigbor_water = self.offsets[neighbor] != self.offsets[neighbor + 1];

            if is_neigbor_water && distance < min_distance {
                best_neighbor = neighbor;
                min_distance = distance;
            }
        }

        if min_distance == u32::MAX {
            return None;
        }
        Some(best_neighbor)
    }

    pub fn new_from_binfile(filename: &str) -> Self {
        println!("Creating Graph from binary file: {}", filename);
        let mut buf_reader = BufReader::new(File::open(&filename).unwrap());
        let graph: Self = bincode::deserialize_from(&mut buf_reader).unwrap();
        println!("Created Graph");
        graph
    }

    pub fn write_to_binfile(&self, filename: &str) {
        println!("Saving Graph to binary file: {}", filename);
        let mut buf_writer = BufWriter::new(File::create(&filename).unwrap());
        bincode::serialize_into(&mut buf_writer, &self).unwrap();
    }

    pub fn get_lon(&self, i: usize) -> f64 {
        let step_size = (360_0000000.0 / self.raster_columns_count as f64) as usize;
        let coordinate = (i % self.raster_columns_count) * step_size;
        let coordinate = coordinate as f64 / FACTOR;
        coordinate - 180.0
    }

    pub fn get_lat(&self, i: usize) -> f64 {
        let step_size = (180_0000000.0 / self.raster_rows_count as f64) as usize;
        let coordinate = (i / self.raster_columns_count) * step_size;
        let coordinate = coordinate as f64 / FACTOR;
        90.0 - coordinate
    }

    pub fn calculate_distance(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> u32 {
        let plon_rad = lon1.to_radians();
        let plat_rad = lat1.to_radians();
        let qlon_rad = lon2.to_radians();
        let qlat_rad = lat2.to_radians();

        (6371000.0
            * f64::acos(
                plat_rad.cos() * qlat_rad.cos() * (plon_rad - qlon_rad).cos()
                    + plat_rad.sin() * qlat_rad.sin(),
            )) as u32
    }

    pub fn generate_random_water_nodes(&self, amount: usize) -> Vec<(usize, usize)> {
        let mut water_nodes = Vec::new();
        for i in 0..(self.raster_rows_count * self.raster_columns_count) {
            if self.get_lat(i) < -83.0 || self.get_lat(i) > 85.01 {
                // Ignore points outside of WGS84
                continue;
            }
            if self.offsets[i] != self.offsets[i + 1] {
                water_nodes.push(i);
            }
        }

        let mut rng = rand::thread_rng();

        let mut chosen_nodes = Vec::new();
        for _ in 0..amount {
            chosen_nodes.push((
                water_nodes[rng.gen_range(0..water_nodes.len())],
                water_nodes[rng.gen_range(0..water_nodes.len())],
            ));
        }
        chosen_nodes
    }

    //
    // Path search algorithm variants
    //

    pub fn dijkstra(&self, start: usize, end: usize, state: &mut AlgorithmState) -> PathResult {
        state.reset();

        state.distances[start] = 0;
        state.queue.push(HeapNode {
            id: start as u32,
            distance: 0,
        });

        let mut heap_pops: usize = 0;
        while let Some(node) = state.queue.pop() {
            heap_pops += 1;

            if node.id as usize == end {
                let mut nodes = Vec::new();
                let mut node = end;
                while node != start {
                    nodes.push(node);
                    node = state.parent_nodes[node] as usize;
                }
                nodes.push(start);
                return PathResult {
                    path: Some(nodes),
                    distance: Some(state.distances[end]),
                    heap_pops,
                };
            }

            for i in self.offsets[node.id as usize].0 as usize
                ..self.offsets[node.id as usize + 1].0 as usize
            {
                let dest = self.edges[i].destination;
                let dist = self.edges[i].distance;
                let new_distance = state.distances[node.id as usize] + dist;

                if new_distance < state.distances[dest as usize] {
                    state.queue.push(HeapNode {
                        id: dest,
                        distance: new_distance,
                    });
                    state.distances[dest as usize] = new_distance;
                    state.parent_nodes[dest as usize] = node.id;
                }
            }
        }

        // No path found
        PathResult {
            path: None,
            distance: None,
            heap_pops,
        }
    }

    pub fn bi_dijkstra(&self, start: usize, end: usize, state: &mut AlgorithmState) -> PathResult {
        state.reset();
        let mut shortest_distance = std::u32::MAX;
        let mut middle_node = 0;

        state.distances[start] = 0;
        state.queue.push(HeapNode {
            id: start as u32,
            distance: 0,
        });
        state.distances2[end] = 0;
        state.queue2.push(HeapNode {
            id: end as u32,
            distance: 0,
        });

        let mut heap_pops: usize = 0;
        while let Some(node) = state.queue.pop() {
            let node2 = state.queue2.pop();
            if node2.is_none() {
                break;
            }
            let node2 = node2.unwrap();

            heap_pops += 2;

            if state.distances[node.id as usize] + state.distances2[node2.id as usize]
                >= shortest_distance
            {
                let mut nodes = Vec::new();
                let mut n = middle_node;
                while n != end {
                    nodes.push(n);
                    n = state.parent_nodes2[n] as usize;
                }
                nodes.push(end);
                nodes.reverse();

                n = state.parent_nodes[middle_node] as usize;
                while n != start {
                    nodes.push(n);
                    n = state.parent_nodes[n] as usize;
                }

                nodes.push(start);
                return PathResult {
                    path: Some(nodes),
                    distance: Some(state.distances[middle_node] + state.distances2[middle_node]),
                    heap_pops,
                };
            }

            for i in self.offsets[node.id as usize].0 as usize
                ..self.offsets[node.id as usize + 1].0 as usize
            {
                let dest = self.edges[i].destination;
                let dist = self.edges[i].distance;
                let new_distance = state.distances[node.id as usize] + dist;

                if new_distance < state.distances[dest as usize] {
                    state.queue.push(HeapNode {
                        id: dest,
                        distance: new_distance,
                    });
                    state.distances[dest as usize] = new_distance;
                    state.parent_nodes[dest as usize] = node.id;

                    if state.distances2[dest as usize] != std::u32::MAX {
                        let d = state.distances[node.id as usize]
                            + dist
                            + state.distances2[dest as usize];
                        if d < shortest_distance {
                            shortest_distance = d;
                            middle_node = dest as usize;
                        }
                    }
                }
            }

            for i in self.offsets[node2.id as usize].0 as usize
                ..self.offsets[node2.id as usize + 1].0 as usize
            {
                let dest = self.edges[i].destination;
                let dist = self.edges[i].distance;
                let new_distance = state.distances2[node2.id as usize] + dist;

                if new_distance < state.distances2[dest as usize] {
                    state.queue2.push(HeapNode {
                        id: dest,
                        distance: new_distance,
                    });
                    state.distances2[dest as usize] = new_distance;
                    state.parent_nodes2[dest as usize] = node2.id;

                    if state.distances[dest as usize] != std::u32::MAX {
                        let d = state.distances[dest as usize]
                            + dist
                            + state.distances2[node2.id as usize];
                        if d < shortest_distance {
                            shortest_distance = d;
                            middle_node = dest as usize;
                        }
                    }
                }
            }
        }

        // No path found
        PathResult {
            path: None,
            distance: None,
            heap_pops,
        }
    }

    pub fn a_star(&self, start: usize, end: usize, state: &mut AlgorithmState) -> PathResult {
        let end_lon = self.get_lon(end);
        let end_lat = self.get_lat(end);
        state.reset();

        state.distances[start] = 0;
        state.queue.push(HeapNode {
            id: start as u32,
            distance: 0,
        });

        let mut heap_pops: usize = 0;
        while let Some(node) = state.queue.pop() {
            heap_pops += 1;

            if node.id == end as u32 {
                let mut nodes = Vec::new();
                let mut current_node = end;
                while current_node != start {
                    nodes.push(current_node);
                    current_node = state.parent_nodes[current_node] as usize;
                }
                nodes.push(start);
                return PathResult {
                    path: Some(nodes),
                    distance: Some(state.distances[end]),
                    heap_pops,
                };
            }

            for i in self.offsets[node.id as usize].0 as usize
                ..self.offsets[node.id as usize + 1].0 as usize
            {
                let dest = self.edges[i].destination as usize;
                let dist = self.edges[i].distance;
                let g_value = state.distances[node.id as usize] + dist;

                if g_value < state.distances[dest] {
                    state.parent_nodes[dest] = node.id;
                    state.distances[dest] = g_value;

                    state.queue.push(HeapNode {
                        id: dest as u32,
                        distance: g_value
                            + Self::calculate_distance(
                                self.get_lon(dest),
                                self.get_lat(dest),
                                end_lon,
                                end_lat,
                            ),
                    });
                }
            }
        }

        // No path found
        PathResult {
            path: None,
            distance: None,
            heap_pops,
        }
    }

    pub fn shortcut_a_star(
        &self,
        start: usize,
        end: usize,
        state: &mut AlgorithmState,
    ) -> PathResult {
        let end_lon = self.get_lon(end);
        let end_lat = self.get_lat(end);
        state.reset();

        state.distances[start] = 0;
        state.queue.push(HeapNode {
            id: start as u32,
            distance: 0,
        });

        // Determine start/end shortcut rectangle beforehand (if they are in one)
        let mut start_rect = self.shortcut_rectangles.len();
        let mut end_rect = self.shortcut_rectangles.len();
        for (i, rect) in self.shortcut_rectangles.iter().enumerate() {
            if self.is_node_inside_rect(start, rect) {
                start_rect = i;
            }
            if self.is_node_inside_rect(end, rect) {
                end_rect = i;
            }
        }

        let mut heap_pops: usize = 0;
        while let Some(node) = state.queue.pop() {
            heap_pops += 1;

            if node.id == end as u32 {
                let mut nodes = Vec::new();
                let mut current_node = end;
                while current_node != start {
                    nodes.push(current_node);
                    current_node = state.parent_nodes[current_node] as usize;
                }
                nodes.push(start);
                return PathResult {
                    path: Some(nodes),
                    distance: Some(state.distances[end]),
                    heap_pops,
                };
            }

            for i in self.offsets[node.id as usize].0 as usize
                ..self.offsets[node.id as usize + 1].0 as usize
            {
                let dest = self.edges[i].destination as usize;
                let dist = self.edges[i].distance;
                let g_value = state.distances[node.id as usize] + dist;

                if g_value < state.distances[dest] {
                    // Skip neighbor if it is inside a shortcut rectangle and the start/end node are not inside the rectangle
                    let rect = self.offsets[dest].1;
                    if rect.is_some() && rect.unwrap() != start_rect && rect.unwrap() != end_rect {
                        continue;
                    }

                    state.parent_nodes[dest] = node.id;
                    state.distances[dest] = g_value;

                    state.queue.push(HeapNode {
                        id: dest as u32,
                        distance: g_value
                            + Self::calculate_distance(
                                self.get_lon(dest),
                                self.get_lat(dest),
                                end_lon,
                                end_lat,
                            ),
                    });
                }
            }
        }

        // No path found
        PathResult {
            path: None,
            distance: None,
            heap_pops,
        }
    }
}
