use std::{
    fs::File,
    io::{BufReader, BufWriter},
};

const FACTOR: f64 = 10_000_000.0;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Edge {
    pub destination: u32,
    pub distance: f32,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Graph {
    pub offsets: Vec<u32>,
    pub edges: Vec<Edge>,
    pub raster_colums_count: usize,
    pub raster_rows_count: usize,
}

impl Graph {
    pub fn find_path(&self, lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> String {
        let nearest_start_node = self.find_nearest_node(lon1, lat1);
        let nearest_end_node = self.find_nearest_node(lon2, lat2);

        let path = Graph::dijkstra(nearest_start_node, nearest_end_node);

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

        String::new()
    }

    pub fn find_nearest_node(&self, lon: f64, lat: f64) -> usize {
        let mut min_distance = f64::MAX;
        let mut node = 0;

        for (i, offset) in self.offsets.iter().enumerate() {
            let next_offset;
            if i == self.offsets.len() - 1 {
                next_offset = self.edges.len() as u32;
            } else {
                next_offset = self.offsets[i + 1];
            }

            // Skip if node is not in water
            if *offset == next_offset {
                continue;
            }

            let distance = Self::calculate_distance(lon, lat, self.get_lon(i), self.get_lat(i));
            if distance < min_distance {
                min_distance = distance;
                node = i;
            }
        }

        node
    }

    pub fn dijkstra(start: usize, end: usize) -> Vec<usize> {
        let nodes = Vec::new();

        nodes
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
        let mut coordinate = coordinate as f64 / FACTOR;
        if coordinate > 180.0 {
            coordinate = coordinate - 360.0;
        }
        coordinate
    }

    pub fn get_lat(&self, i: usize) -> f64 {
        let step_size = (180_0000000.0 / self.raster_rows_count as f64) as usize;
        let coordinate = (i / self.raster_colums_count) * step_size;
        let mut coordinate = coordinate as f64 / FACTOR;
        if coordinate > 90.0 {
            coordinate = coordinate - 180.0;
        }
        coordinate
    }

    pub fn calculate_distance(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
        let plon_rad = (lon1).to_radians();
        let plat_rad = (lat1).to_radians();
        let qlon_rad = (lon2).to_radians();
        let qlat_rad = (lat2).to_radians();

        let lat_diff = qlat_rad - plat_rad;
        let lon_diff = qlon_rad - plon_rad;

        let a = (lat_diff / 2.0).sin() * (lat_diff / 2.0).sin()
            + plat_rad.cos() * qlat_rad.cos() * (lon_diff / 2.0).sin() * (lon_diff / 2.0).sin();
        let c = 2.0 * f64::atan2(a.sqrt(), (1.0 - a).sqrt());

        6371.0 * c
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
