use std::time::Instant;
use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    fs::File,
    io::{BufReader, BufWriter},
};

use rand::Rng;

const FACTOR: f64 = 10_000_000.0;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Edge {
    pub destination: u32,
    pub distance: u32,
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Graph {
    pub offsets: Vec<u32>,
    pub edges: Vec<Edge>,
    pub raster_colums_count: usize,
    pub raster_rows_count: usize,
}

pub struct PathResult {
    pub path: Option<Vec<usize>>,
    pub distance: Option<u32>,
    pub heap_pops: usize,
}

impl Graph {
    pub fn find_path(
        &self,
        lon1: f64,
        lat1: f64,
        lon2: f64,
        lat2: f64,
    ) -> Option<(GEOJson<Vec<[f64; 2]>>, f64)> {
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
            let result = self.a_star(nearest_start_node, nearest_end_node);
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

        Some((geojson, distance as f64))
    }

    pub fn find_nearest_node(&self, lon: f64, lat: f64) -> Option<usize> {
        let step_size_lon = (360_0000000.0 / self.raster_colums_count as f64) as usize;
        let lon_index_left = ((lon + 180.) * FACTOR) as usize / step_size_lon;
        let lon_index_right = (lon_index_left + 1) % self.raster_colums_count;

        let step_size_lat = (180_0000000.0 / self.raster_rows_count as f64) as usize;
        let lat_index_top = ((lat + 90.) * FACTOR) as usize / step_size_lat;
        let lat_index_bottom = (lat_index_top + 1) % self.raster_rows_count;

        let neighbor_ids = vec![
            (lat_index_top * self.raster_colums_count) + lon_index_left,
            (lat_index_top * self.raster_colums_count) + lon_index_right,
            (lat_index_bottom * self.raster_colums_count) + lon_index_left,
            (lat_index_bottom * self.raster_colums_count) + lon_index_right,
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

    pub fn dijkstra(&self, start: usize, end: usize) -> PathResult {
        let mut nodes = Vec::new();

        let node_count = self.raster_colums_count * self.raster_rows_count;

        let mut distances = vec![std::u32::MAX; node_count];
        let mut parent_nodes = vec![std::u32::MAX; node_count];

        let mut queue = BinaryHeap::with_capacity(node_count);

        distances[start] = 0;
        queue.push(HeapNode {
            id: start as u32,
            distance: 0,
        });

        let mut heap_pops: usize = 0;
        while let Some(node) = queue.pop() {
            heap_pops += 1;

            for i in
                self.offsets[node.id as usize] as usize..self.offsets[node.id as usize + 1] as usize
            {
                let dest = self.edges[i].destination;
                let dist = self.edges[i].distance;
                let new_distance = distances[node.id as usize] + dist;

                if new_distance < distances[dest as usize] {
                    queue.push(HeapNode {
                        id: dest,
                        distance: new_distance,
                    });
                    distances[dest as usize] = new_distance;
                    parent_nodes[dest as usize] = node.id;
                    if dest as usize == end {
                        // return if a path to the end is found
                        let mut node = end;
                        while node != start {
                            nodes.push(node);
                            node = parent_nodes[node] as usize;
                        }
                        nodes.push(start);
                        return PathResult {
                            path: Some(nodes),
                            distance: Some(distances[end]),
                            heap_pops,
                        };
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

    pub fn a_star(&self, start: usize, end: usize) -> PathResult {
        let end_lon = self.get_lon(end);
        let end_lat = self.get_lat(end);
        let mut nodes = Vec::new();

        let node_count = self.raster_colums_count * self.raster_rows_count;

        let mut previous_node = vec![std::u32::MAX; node_count];
        let mut g_values = vec![std::u32::MAX; node_count];

        let mut queue = BinaryHeap::with_capacity(node_count);
        g_values[start] = 0;
        queue.push(HeapNode {
            id: start as u32,
            distance: 0,
        });

        let mut heap_pops: usize = 0;
        while let Some(node) = queue.pop() {
            heap_pops += 1;

            if node.id == end as u32 {
                let mut current_node = end;
                while current_node != start {
                    nodes.push(current_node);
                    current_node = previous_node[current_node] as usize;
                }
                nodes.push(start);
                return PathResult {
                    path: Some(nodes),
                    distance: Some(g_values[end]),
                    heap_pops,
                };
            }

            for i in
                self.offsets[node.id as usize] as usize..self.offsets[node.id as usize + 1] as usize
            {
                let dest = self.edges[i].destination as usize;
                let dist = self.edges[i].distance;
                let g_value = g_values[node.id as usize] + dist;

                if g_value < g_values[dest] {
                    previous_node[dest] = node.id;
                    g_values[dest] = g_value;

                    queue.push(HeapNode {
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
        let step_size = (360_0000000.0 / self.raster_colums_count as f64) as usize;
        let coordinate = (i % self.raster_colums_count) * step_size;
        let coordinate = coordinate as f64 / FACTOR;
        coordinate - 180.0
    }

    pub fn get_lat(&self, i: usize) -> f64 {
        let step_size = (180_0000000.0 / self.raster_rows_count as f64) as usize;
        let coordinate = (i / self.raster_colums_count) * step_size;
        let coordinate = coordinate as f64 / FACTOR;
        coordinate - 90.0
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
        for i in 0..(self.raster_rows_count * self.raster_colums_count) {
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
