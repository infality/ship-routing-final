use std::time::Instant;
use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    fs::File,
    io::{BufReader, BufWriter},
};

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

impl Graph {
    pub fn find_path(
        &self,
        lon1: f64,
        lat1: f64,
        lon2: f64,
        lat2: f64,
    ) -> (GEOJson<Vec<[f64; 2]>>, f64) {
        let mut now = Instant::now();
        let nearest_start_node_opt = self.find_nearest_node(lon1, lat1);
        let nearest_end_node_opt = self.find_nearest_node(lon2, lat2);
        println!(
            "Time taken for nearest node search: {}ms",
            now.elapsed().as_micros() as f32 / 1000.
        );
        now = Instant::now();

        let mut coordinates = Vec::<[f64; 2]>::new();
        coordinates.push([lon2, lat2]);
        let mut distance = 0;

        if nearest_start_node_opt != None && nearest_end_node_opt != None {
            let nearest_start_node = nearest_start_node_opt.unwrap();
            let nearest_end_node = nearest_end_node_opt.unwrap();
                
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
                println!("start node is equal to end node. skipping dijkstra");
                distance += Self::calculate_distance(lon1, lat1, lon2, lat2);
            } else {
                println!("start node is not equal to end node. executing dijkstra");
                let result = self.dijkstra(nearest_start_node, nearest_end_node);
                if result.is_none() {
                    println!("dijkstra did not find a route");
                    distance += Self::calculate_distance(lon1, lat1, lon2, lat2);
                } else {
                    let (path, d) = result.unwrap();
                    distance += d;
                    println!("Path length: {}", path.len());
                    for node in path.iter() {
                        coordinates.push([self.get_lon(*node), self.get_lat(*node)]);
                    }

                    distance += Self::calculate_distance(
                        lon1,
                        lat1,
                        self.get_lon(path[0]),
                        self.get_lat(path[0]),
                    );
                    distance += Self::calculate_distance(
                        self.get_lon(*path.last().unwrap()),
                        self.get_lat(*path.last().unwrap()),
                        lon2,
                        lat2,
                    );
                }
            }
            println!(
                "Time taken for path search: {}ms",
                now.elapsed().as_micros() as f32 / 1000.
            );
        }

        coordinates.push([lon1, lat1]);

        let mut geojson = GEOJson {
            r#type: "FeatureCollection",
            features: Vec::new(),
        };

        geojson.features.push(GEOJsonFeature {
            r#type: "Feature",
            geometry: GEOJsonGeometry {
                r#type: "LineString",
                coordinates: coordinates.clone(),
            },
            properties: GEOJsonProperty {},
        });

        //for coordinate in coordinates {
        //    geojson.features.push(GEOJsonFeature {
        //        r#type: "Feature",
        //        geometry: GEOJsonGeometry {
        //            r#type: "Point",
        //            coordinates: coordinate.clone(),
        //        },
        //        properties: GEOJsonProperty {},
        //    });
        //}

        (geojson, distance as f64)
    }

    pub fn find_nearest_node(&self, lon: f64, lat: f64) -> Option<usize> {
        //let mut min_distance = u32::MAX;
        //let mut node = 0;

        //for i in 0..self.raster_rows_count * self.raster_colums_count {
        //    let offset = self.offsets[i];
        //    let next_offset = self.offsets[i + 1];

        //    // Skip if node is not in water
        //    if offset == next_offset {
        //        continue;
        //    }

        //    let distance = Self::calculate_distance(lon, lat, self.get_lon(i), self.get_lat(i));
        //    if distance < min_distance {
        //        min_distance = distance;
        //        node = i;
        //    }
        //}
        let step_size_lon = (360_0000000.0 / self.raster_colums_count as f64) as usize;
        let lon_index_left = ((lon + 180.) * FACTOR) as usize / step_size_lon;
        let lon_index_right = (lon_index_left + 1) % self.raster_rows_count;

        let step_size_lat = (180_0000000.0 / self.raster_rows_count as f64) as usize;
        let lat_index_top = ((lat + 90.) * FACTOR) as usize / step_size_lat;
        let lat_index_bottom = (lat_index_top + 1) % self.raster_colums_count;

        let mut neighbor_ids = vec![];
        neighbor_ids.push((lat_index_top * self.raster_colums_count) + lon_index_left);
        neighbor_ids.push((lat_index_top * self.raster_colums_count) + lon_index_right);
        neighbor_ids.push((lat_index_bottom * self.raster_colums_count) + lon_index_left);
        neighbor_ids.push((lat_index_bottom * self.raster_colums_count) + lon_index_right);

        let mut best_neighbor = neighbor_ids[0];
        let mut min_distance = u32::MAX;
        for neighbor in neighbor_ids {
            let distance = Self::calculate_distance(lon, lat, self.get_lon(neighbor), self.get_lat(neighbor));
            if distance < min_distance {
                // TODO check if neighbor is in water
                best_neighbor = neighbor;
                min_distance = distance;
            }
        }

        if min_distance == u32::MAX {
            return None;
        }
        Some(best_neighbor)
    }

    pub fn dijkstra(&self, start: usize, end: usize) -> Option<(Vec<usize>, u32)> {
        let mut nodes = Vec::new();

        let node_count = self.raster_colums_count * self.raster_rows_count;

        let mut distances: Vec<u32> = vec![std::u32::MAX; node_count];
        let mut parent_nodes: Vec<u32> = vec![std::u32::MAX; node_count];
        let mut finished: Vec<bool> = vec![false; node_count];

        let mut queue = BinaryHeap::with_capacity(node_count);

        distances[start] = 0;
        queue.push(HeapNode {
            id: start as u32,
            distance: 0,
        });

        while let Some(node) = queue.pop() {
            if finished[node.id as usize] {
                continue;
            }
            finished[node.id as usize] = true;

            for i in
                self.offsets[node.id as usize] as usize..self.offsets[node.id as usize + 1] as usize
            {
                let dest = self.edges[i].destination;
                let dist = self.edges[i].distance;

                if !finished[dest as usize] {
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
                            eprintln!("Calculated one-to-one-dijkstra");
                            return Some((nodes, distances[end]));
                        }
                    }
                }
            }
        }

        // No path found
        None
    }

    pub fn new_from_binfile(filename: &str) -> Self {
        println!("Creating Graph from binary file: {}", filename);
        let mut buf_reader = BufReader::new(File::open(&filename).unwrap());
        let graph: Self = bincode::deserialize_from(&mut buf_reader).unwrap();
        println!("Created Graph");
        return graph;
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
        let plon_rad = (lon1).to_radians();
        let plat_rad = (lat1).to_radians();
        let qlon_rad = (lon2).to_radians();
        let qlat_rad = (lat2).to_radians();

        let lat_diff = qlat_rad - plat_rad;
        let lon_diff = qlon_rad - plon_rad;

        let a = (lat_diff / 2.0).sin() * (lat_diff / 2.0).sin()
            + plat_rad.cos() * qlat_rad.cos() * (lon_diff / 2.0).sin() * (lon_diff / 2.0).sin();
        let c = 2.0 * f64::atan2(a.sqrt(), (1.0 - a).sqrt());

        (6371.0 * c) as u32
    }

    /* fn calculate_distance2(&self, p: usize, q: usize) -> f64 {
        let plon_rad = p.get_lon().to_radians();
        let plat_rad = p.get_lat().to_radians();
        let qlon_rad = q.get_lon().to_radians();
        let qlat_rad = q.get_lat().to_radians();

        let p_vec = [
            plat_rad.cos() * plon_rad.cos(),
            plat_rad.cos() * plon_rad.sin(),
            plat_rad.sin(),
        ];

        let q_vec = [
            qlat_rad.cos() * qlon_rad.cos(),
            qlat_rad.cos() * qlon_rad.sin(),
            qlat_rad.sin(),
        ];

        let cross = [
            p_vec[1] * q_vec[2] - p_vec[2] * q_vec[1],
            p_vec[2] * q_vec[0] - p_vec[0] * q_vec[2],
            p_vec[0] * q_vec[1] - p_vec[1] * q_vec[0],
        ];
        let cross_length =
            f64::sqrt(cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]);
        let dot = p_vec[0] * q_vec[0] + p_vec[1] * q_vec[1] + p_vec[2] * q_vec[2];

        6371.0 * f64::atan2(cross_length, dot)
    } */
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
