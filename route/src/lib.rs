use std::{
    fs::File,
    io::{BufReader, BufWriter},
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Graph {
    pub nodes_is_water: Vec<bool>,
    pub offsets: Vec<u32>,
    pub edges: Vec<u32>,
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

    fn is_water(&self, i: usize) -> bool {
        return self.nodes_is_water[i as usize];
    }

    fn get_neighbour_top(&self, i: usize) -> usize {
        return (i + 1) % (self.raster_colums_count * self.raster_rows_count);
    }
    fn get_neighbour_bottom(&self, i: usize) -> usize {
        return (i - 1) % (self.raster_colums_count * self.raster_rows_count);
    }
    fn get_neighbour_right(&self, i: usize) -> usize {
        return (i + self.raster_rows_count) % (self.raster_colums_count * self.raster_rows_count);
    }
    fn get_neighbour_left(&self, i: usize) -> usize {
        return (i - self.raster_rows_count) % (self.raster_colums_count * self.raster_rows_count);
    }

    fn get_neighbours_in_water(&self, i: usize) -> Vec<usize> {
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

    fn get_distance(&self, i: usize, j: usize) -> f64 {
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
}
