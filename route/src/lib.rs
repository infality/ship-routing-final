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
    pub nodes_is_water: Vec<bool>,
    pub offsets: Vec<u32>,
    pub edges: Vec<Edge>,
    pub raster_colums_count: usize,
    pub raster_rows_count: usize,
}

impl Graph {
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

    pub fn is_water(&self, i: usize) -> bool {
        return self.nodes_is_water[i];
    }

    pub fn get_lon(&self, i: usize) -> i32 {
        let step_size = (360_0000000.0 / self.raster_colums_count as f64) as usize;
        (i * step_size) as i32 - 180_0000000
    }

    pub fn get_lat(&self, i: usize) -> i32 {
        let step_size = (180_0000000.0 / self.raster_rows_count as f64) as usize;
        (i * step_size) as i32 - 90_0000000
    }

    pub fn get_neighbour_top(&self, i: usize) -> usize {
        if i < self.raster_rows_count {
            return (i + self.raster_rows_count) % self.raster_rows_count;
        }
        return i - self.raster_rows_count;
    }
    pub fn get_neighbour_bottom(&self, i: usize) -> usize {
        if i >= self.raster_colums_count * (self.raster_rows_count - 1) {
            return self.raster_colums_count * (self.raster_rows_count - 1)
                + ((i + self.raster_rows_count) % self.raster_rows_count);
        }
        return i + self.raster_rows_count;
    }
    pub fn get_neighbour_right(&self, i: usize) -> usize {
        let row = i / self.raster_colums_count;
        return row * self.raster_colums_count + ((i + 1) % self.raster_colums_count);
    }
    pub fn get_neighbour_left(&self, i: usize) -> usize {
        let row = i / self.raster_colums_count;
        return row * self.raster_colums_count + ((i - 1) % self.raster_colums_count);
    }

    pub fn get_neighbours_in_water(&self, i: usize) -> Vec<usize> {
        let mut neighbours = Vec::new();
        // TODO is there a performance impact if we iterate over a vec of neighbours instead?
        let top = self.get_neighbour_top(i);
        let bottom = self.get_neighbour_bottom(i);
        let right = self.get_neighbour_right(i);
        let left = self.get_neighbour_left(i);
        if self.is_water(top) {
            neighbours.push(top);
        }
        if self.is_water(bottom) {
            neighbours.push(bottom);
        }
        if self.is_water(right) {
            neighbours.push(right);
        }
        if self.is_water(left) {
            neighbours.push(left);
        }
        return neighbours;
    }

    pub fn get_distance(&self, i: usize, j: usize) -> f64 {
        // this function ONLY works for direct neighbours!
        // TODO does this substraction crash with usize?
        if i - j == 1 || j - i == 1 {
            // top or bottom neighbour
            // assuming an earth radius of 1
            return std::f64::consts::PI / 180.;
        } else {
            // right or left neighbour
            let lat =
                (i % self.raster_colums_count) as f64 / (self.raster_rows_count * 180) as f64 - 90.;
            // TODO this distance depends on the latitude we are currently on and we wan to assume an earth radius of 1
            // TODO maybe use a lookup table for this based on the current row_number which is (i % self.raster_colums_count)
            // TODO maybe (https://en.wikipedia.org/wiki/Haversine_formula)
            // assuming an earth radius of 1
            return 1.337;
        }
    }

    pub fn calculate_distance(&self, p: usize, q: usize) -> f64 {
        let plon_rad = (FACTOR * self.get_lon(p) as f64).to_radians();
        let plat_rad = (FACTOR * self.get_lat(p) as f64).to_radians();
        let qlon_rad = (FACTOR * self.get_lon(q) as f64).to_radians();
        let qlat_rad = (FACTOR * self.get_lat(q) as f64).to_radians();

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
