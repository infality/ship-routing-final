use rayon::prelude::*;
use route::{Edge, Graph};
use std::sync::atomic::AtomicUsize;
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::BufReader,
    io::BufWriter,
    io::Error,
};

const GRAPH_ROWS_COUNT: usize = 850;
const GRAPH_COLUMNS_COUNT: usize = 1250;
const FACTOR_INT: i32 = 10_000_000;
const FACTOR: f64 = 10_000_000.0;
const WATER: Coordinate = Coordinate {
    lat: 90 * FACTOR_INT,
    lon: 0,
};

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
struct Coordinate {
    lon: i32,
    lat: i32,
}

impl Coordinate {
    fn is_equal(&self, other: &Coordinate) -> bool {
        return self.lon == other.lon && self.lat == other.lat;
    }

    fn get_lat(&self) -> f64 {
        self.lat as f64 / FACTOR
    }

    fn get_lon(&self) -> f64 {
        self.lon as f64 / FACTOR
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Coast {
    coordinates: Vec<Coordinate>,
    leftmost: i32,
    rightmost: i32,
}

impl Coast {
    fn get_first(&self) -> Coordinate {
        return self.coordinates.first().unwrap().clone();
    }
    fn get_last(&self) -> Coordinate {
        return self.coordinates.last().unwrap().clone();
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Coasts {
    actual_coasts: Vec<Coast>,
}

impl Coasts {
    fn new_from_pbffile(filename: &str) -> Self {
        println!("Creating Coasts from pbf file: {}", filename);
        let file = File::open(&filename).unwrap();
        let reader = BufReader::new(file);

        let mut pbf = osmpbfreader::OsmPbfReader::new(reader);

        let mut nodes = HashMap::<i64, Coordinate>::with_capacity(63000000);
        let mut coasts = HashMap::<Coordinate, Coast>::with_capacity(1000000);

        let mut counter = 0;
        for obj in pbf.iter() {
            if counter % 1000000 == 0 {
                println!("Searching nodes: {}", counter);
            }
            counter += 1;

            match obj {
                Ok(osmpbfreader::OsmObj::Node(n)) => drop(nodes.insert(
                    n.id.0,
                    Coordinate {
                        lon: n.decimicro_lon,
                        lat: n.decimicro_lat,
                    },
                )),
                Ok(osmpbfreader::OsmObj::Way(w)) => {
                    let mut coordinates = Vec::<Coordinate>::with_capacity(w.nodes.len());
                    let mut leftmost = i32::MAX;
                    let mut rightmost = i32::MIN;
                    for node in w.nodes.iter() {
                        let n = nodes.get(&node.0).unwrap().clone();
                        if n.lon < leftmost {
                            leftmost = n.lon;
                        }
                        if n.lon > rightmost {
                            rightmost = n.lon;
                        }
                        coordinates.push(n);
                    }
                    coasts.insert(
                        coordinates.first().unwrap().clone(),
                        Coast {
                            coordinates,
                            leftmost,
                            rightmost,
                        },
                    );
                }
                _ => continue,
            }
        }

        println!("Found {} nodes", nodes.len());
        println!("Found {} ways", coasts.len());
        println!("Finished parsing");

        let mut actual_coasts = Vec::<Coast>::new();
        let mut current_coast;
        {
            let first_key = coasts.keys().next().unwrap().clone();
            let first_coast = coasts.remove(&first_key).unwrap();
            current_coast = first_coast
        }

        counter = 1;
        loop {
            while !current_coast
                .get_first()
                .is_equal(&current_coast.get_last())
            {
                let coordinate = current_coast.get_last();
                if let Some(coast) = coasts.get_mut(&coordinate) {
                    counter += 1;
                    if counter % 1000 == 0 {
                        println!("Merged coasts: {}", counter);
                    }
                    current_coast.coordinates.append(&mut coast.coordinates);

                    if coast.leftmost < current_coast.leftmost {
                        current_coast.leftmost = coast.leftmost;
                    }
                    if coast.rightmost > current_coast.rightmost {
                        current_coast.rightmost = coast.rightmost;
                    }

                    coasts.remove(&coordinate);
                }
            }

            actual_coasts.push(current_coast);

            let next_key = coasts.keys().next();
            if next_key.is_none() {
                break;
            }
            let next_key = next_key.unwrap().clone();
            let next_coast = coasts.remove(&next_key).unwrap();
            counter += 1;
            if counter % 1000 == 0 {
                println!("Merged coasts: {}", counter);
            }

            current_coast = next_coast;
        }

        println!("Found {} actual coasts", actual_coasts.len());
        println!("Finished merging");

        println!("Created {} Coasts", actual_coasts.len());
        return Coasts { actual_coasts };
    }

    fn new_from_binfile(filename: &str) -> Self {
        println!("Creating Coasts from bin file: {}", filename);
        let mut buf_reader = BufReader::new(File::open(&filename).unwrap());
        let coasts: Self = bincode::deserialize_from(&mut buf_reader).unwrap();
        println!("Created {} Coasts", coasts.actual_coasts.len());
        return coasts;
    }

    fn write_to_binfile(&self, filename: &str) {
        println!("Saving Coasts to binary file: {}", filename);
        let mut buf_writer = BufWriter::new(File::create(&filename).unwrap());
        bincode::serialize_into(&mut buf_writer, &self.actual_coasts).unwrap();
    }

    fn write_to_geojson(&self, filename: &str) {
        println!("Saving Coasts to geojson file: {}", filename);
        let mut geo_json = route::GEOJson {
            r#type: "FeatureCollection",
            features: Vec::new(),
        };

        for actual_coast in self.actual_coasts.iter() {
            let mut coordinates = Vec::<[f64; 2]>::new();

            for coordinate in actual_coast.coordinates.iter().rev() {
                coordinates.push([coordinate.get_lon(), coordinate.get_lat()]);
            }

            geo_json.features.push(route::GEOJsonFeature {
                r#type: "Feature",
                geometry: route::GEOJsonGeometry {
                    r#type: "Polygon",
                    coordinates: [coordinates],
                },
                properties: route::GEOJsonProperty {},
            });
        }

        let output_json = serde_json::to_string(&geo_json).unwrap();
        fs::write(&filename, output_json).unwrap();
    }
}

struct Node {
    coordinate: Coordinate,
    is_water: bool,
}

impl Node {
    fn set_water_flag(&mut self, coasts: &Coasts) {
        // check if node is on southpole. this is a special case we can't handle with our algorithm
        if self.coordinate.lat == -90 * FACTOR_INT {
            self.is_water = false;
            return;
        }
        for coast in coasts.actual_coasts.iter() {
            if !(coast.leftmost <= self.coordinate.lon && self.coordinate.lon <= coast.rightmost) {
                continue;
            }

            let mut intersection_count = 0;
            for line in 0..coast.coordinates.len() {
                let first = coast.coordinates[line];
                let second = coast.coordinates[(line + 1) % coast.coordinates.len()];

                // continue if line is south of us (works but does not improve performance at all)
                //if first.lat < self.coordinate.lat && second.lat < self.coordinate.lat {
                //    continue;
                //}

                // handle special case if line is vertical
                if first.lon == second.lon {
                    continue;
                }

                // handle special case if node is on the first vertex
                if self.coordinate.lat == first.lat && self.coordinate.lon == first.lon {
                    self.is_water = false;
                    return;
                }

                // continue if our lon is not between the lines small and large lon
                // (correct) assumtion: no line crosses the antimeridian at lon 180 / -180
                let smaller_lon;
                let larger_lon;
                if first.lon <= second.lon {
                    smaller_lon = first.lon;
                    larger_lon = second.lon;
                } else {
                    smaller_lon = second.lon;
                    larger_lon = first.lon;
                }
                if !(smaller_lon <= self.coordinate.lon
                    && (self.coordinate.lon < larger_lon || larger_lon == 180 * FACTOR_INT))
                {
                    // nodes lat is not between the lons of the line
                    continue;
                }

                let tlon_x = transform_lon(&first, &WATER);
                let tlon_second = transform_lon(&first, &second);
                let tlon_self = transform_lon(&first, &self.coordinate);
                if tlon_self == tlon_second {
                    // node is on the line
                    self.is_water = false;
                    return;
                } else {
                    let bearing_second_x = east_or_west(tlon_second, tlon_x);
                    let bearing_second_self = east_or_west(tlon_second, tlon_self);
                    if bearing_second_x == -bearing_second_self {
                        intersection_count += 1;
                    }
                }
            }
            if intersection_count % 2 == 1 {
                self.is_water = false;
                return;
            }
        }
    }
}

struct Nodes {
    nodes: Vec<Node>,
}

impl Nodes {
    fn new_generate_equally_distributed() -> Nodes {
        println!("Generating equally distributed nodes");
        let mut nodes = Vec::new();

        let node_count = 1000;
        let a = 1.0 / node_count as f64;
        let d = f64::sqrt(a);
        let m_theta = f64::round(std::f64::consts::PI / d);
        let d_theta = std::f64::consts::PI / m_theta;
        let d_phi = a / d_theta;

        for m in 0..(m_theta as isize) {
            let theta = std::f64::consts::PI * (m as f64 + 0.5) / m_theta;
            let m_phi = f64::round(2.0 * std::f64::consts::PI * theta.sin() / d_phi);
            for n in 0..(m_phi as isize) {
                let phi = 2.0 * std::f64::consts::PI * n as f64 / m_phi;

                let lat = theta.to_degrees() - 90.0;
                let lon = phi.to_degrees() - 180.0;
                nodes.push(Node {
                    coordinate: Coordinate {
                        lon: (lon * FACTOR) as i32,
                        lat: (lat * FACTOR) as i32,
                    },
                    is_water: true,
                });
            }
        }

        Nodes { nodes }
    }

    fn new_generate_not_equally_distributed() -> Nodes {
        println!("Generating not equally distributed nodes");
        let mut nodes = Vec::new();

        for lat in (-90 * FACTOR_INT..90 * FACTOR_INT)
            .step_by((180.0 * FACTOR / GRAPH_ROWS_COUNT as f64) as usize)
        {
            for lon in (-180 * FACTOR_INT..180 * FACTOR_INT)
                .step_by((360.0 * FACTOR / GRAPH_COLUMNS_COUNT as f64) as usize)
            {
                nodes.push(Node {
                    coordinate: Coordinate { lon, lat },
                    is_water: true,
                });
            }
        }
        Nodes { nodes }
    }

    fn write_to_geojson(&self, filename: &str) {
        println!("Saving Nodes to geojson file: {}", filename);
        let mut geo_json = route::GEOJson {
            r#type: "FeatureCollection",
            features: Vec::new(),
        };

        for node in self.nodes.iter() {
            if !node.is_water {
                continue;
            }
            let coordinates = [
                node.coordinate.lon as f64 / FACTOR,
                node.coordinate.lat as f64 / FACTOR,
            ];

            geo_json.features.push(route::GEOJsonFeature {
                r#type: "Feature",
                geometry: route::GEOJsonGeometry {
                    r#type: "Point",
                    coordinates,
                },
                properties: route::GEOJsonProperty {},
            });
        }

        let output_json = serde_json::to_string(&geo_json).unwrap();
        fs::write(&filename, output_json).unwrap();
    }
}

trait GraphExt {
    fn new_from_nodes(nodes: Nodes, raster_colums_count: usize, raster_rows_count: usize) -> Graph;
    fn get_neighbors(&self, i: usize) -> Vec<usize>;
}

impl GraphExt for Graph {
    fn get_neighbors(&self, i: usize) -> Vec<usize> {
        let mut neighbors = Vec::new();
        let row = i / self.raster_colums_count;

        if row > 0 {
            neighbors.push(i - self.raster_colums_count);
        }
        if row < self.raster_rows_count - 1 {
            neighbors.push(i + self.raster_colums_count);
        }

        neighbors.push(row * self.raster_colums_count + ((i + 1) % self.raster_colums_count));
        neighbors.push(row * self.raster_colums_count + ((i - 1) % self.raster_colums_count));
        neighbors
    }

    fn new_from_nodes(nodes: Nodes, raster_colums_count: usize, raster_rows_count: usize) -> Graph {
        let mut graph = Graph {
            offsets: Vec::new(),
            edges: Vec::new(),
            raster_colums_count,
            raster_rows_count,
        };

        for (i, node) in nodes.nodes.iter().enumerate() {
            graph.offsets.push(graph.edges.len() as u32);
            if !node.is_water {
                continue;
            }

            let neighbors = graph.get_neighbors(i);
            for neighbor in neighbors {
                let distance = Self::calculate_distance(
                    graph.get_lon(i),
                    graph.get_lat(i),
                    graph.get_lon(neighbor),
                    graph.get_lat(neighbor),
                );

                graph.edges.push(Edge {
                    destination: neighbor as u32,
                    distance: distance,
                });
            }
        }

        graph
    }
}

fn transform_lon(p: &Coordinate, q: &Coordinate) -> f64 {
    if p.lat == 90 * FACTOR_INT {
        return q.get_lon();
    } else {
        let plon_rad = p.get_lon().to_radians();
        let plat_rad = p.get_lat().to_radians();
        let qlon_rad = q.get_lon().to_radians();
        let qlat_rad = q.get_lat().to_radians();
        let t = (qlon_rad - plon_rad).sin() * qlat_rad.cos();
        let b = qlat_rad.sin() * plat_rad.cos()
            - qlat_rad.cos() * plat_rad.sin() * (qlon_rad - plon_rad).cos();
        return f64::atan2(t, b).to_degrees();
    }
}

fn east_or_west(clon: f64, dlon: f64) -> i32 {
    let mut del = dlon - clon;
    if del > 180.0 {
        del = del - 360.0;
    } else if del < -180.0 {
        del = del + 360.0;
    }
    if del > 0.0 && del != 180.0 {
        return -1;
    } else if del < 0.0 && del != -180.0 {
        return 1;
    } else {
        return 0;
    }
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();

    let file_name;
    let skip_read_pbf;

    match args.len() {
        l if l < 2 => {
            println!("Please pass a pbf file");
            return Ok(());
        }
        l if l == 2 => {
            file_name = &args[1];
            skip_read_pbf = false;
        }
        l if l == 3 => {
            if &args[1] == "-s" || &args[1] == "--skip-read-pbf" {
                skip_read_pbf = true;
                file_name = &args[2];
            } else if &args[2] == "-s" || &args[2] == "--skip-read-pbf" {
                file_name = &args[1];
                skip_read_pbf = true;
            } else {
                println!("Invalid argument");
                return Ok(());
            }
        }
        _ => {
            println!("Too many arguments");
            return Ok(());
        }
    }

    let coasts;
    if !skip_read_pbf {
        coasts = Coasts::new_from_pbffile(&file_name);
        coasts.write_to_geojson("coastlines.json");
        coasts.write_to_binfile("coastlines.bin");
    } else {
        coasts = Coasts::new_from_binfile(&file_name);
        //coasts.write_to_geojson("coastlines.json");
    }

    let mut nodes = Nodes::new_generate_not_equally_distributed();

    println!("Setting water flags for {} nodes", nodes.nodes.len());
    let counter = AtomicUsize::new(0);
    nodes.nodes.par_iter_mut().for_each(|node| {
        let current_count = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if current_count % 10000 == 0 {
            println!("Progress: {}", current_count);
        }

        node.set_water_flag(&coasts);
    });

    nodes.write_to_geojson("nodes.json");
    let graph = Graph::new_from_nodes(nodes, GRAPH_COLUMNS_COUNT, GRAPH_ROWS_COUNT);
    graph.write_to_binfile("graph.bin");

    Ok(())
}
