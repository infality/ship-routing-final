use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::BufReader,
    io::BufWriter,
    io::Error,
};

const FACTOR: f64 = 10_000_000.0;

#[derive(serde::Serialize)]
struct GEOJson {
    r#type: &'static str,
    features: Vec<GEOJsonFeature>,
}

#[derive(serde::Serialize)]
struct GEOJsonFeature {
    r#type: &'static str,
    geometry: GEOJsonGeometry,
    properties: GEOJsonProperty,
}

#[derive(serde::Serialize)]
struct GEOJsonGeometry {
    r#type: &'static str,
    coordinates: [Vec<[f64; 2]>; 1],
}

#[derive(serde::Serialize)]
struct GEOJsonProperty {}

#[derive(Clone, Copy, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
struct Coordinate {
    lon: i32,
    lat: i32,
}

impl Coordinate {
    fn is_equal(&self, other: &Coordinate) -> bool {
        return self.lon == other.lon && self.lat == other.lat;
    }

    fn get_lon(&self) -> f64 {
        self.lon as f64 / FACTOR
    }

    fn get_lat(&self) -> f64 {
        self.lat as f64 / FACTOR
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Coast {
    coordinates: Vec<Coordinate>,
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
                    for node in w.nodes.iter() {
                        coordinates.push(nodes.get(&node.0).unwrap().clone());
                    }
                    coasts.insert(coordinates.first().unwrap().clone(), Coast { coordinates });
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
        let mut geo_json = GEOJson {
            r#type: "FeatureCollection",
            features: Vec::new(),
        };

        for actual_coast in self.actual_coasts.iter() {
            let mut coordinates = Vec::<[f64; 2]>::new();

            for coordinate in actual_coast.coordinates.iter().rev() {
                coordinates.push([coordinate.get_lon(), coordinate.get_lat()]);
            }

            geo_json.features.push(GEOJsonFeature {
                r#type: "Feature",
                geometry: GEOJsonGeometry {
                    r#type: "Polygon",
                    coordinates: [coordinates],
                },
                properties: GEOJsonProperty {},
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
    fn generate_nodes() -> Vec<Node> {
        let mut nodes = Vec::new();

        // TODO Generate equally distributed nodes
        for lon in -10..=10 {
            for lat in -10..=10 {
                nodes.push(Node {
                    coordinate: Coordinate {
                        lon: lon * FACTOR as i32,
                        lat: lat * FACTOR as i32,
                    },
                    is_water: false,
                });
            }
        }

        nodes
    }

    fn set_water_flag(&mut self, coasts: &Coasts) {
        let mut intersections = 0;
        for coast in coasts.actual_coasts.iter() {
            for line in 0..coast.coordinates.len() {
                let first = coast.coordinates[line];
                let second = coast.coordinates[(line + 1) % coast.coordinates.len()];

                // Handle special case if line is vertical
                if first.lon == second.lon {
                    if first.lon == self.coordinate.lon {
                        intersections += 1;
                    }
                    continue;
                }

                let bearing = calculate_bearing(&first, &second);
                let intersection = calculate_intersection(&self.coordinate, &first, bearing);
                if (first.lon <= intersection.lon && intersection.lon <= second.lon)
                    || (second.lon <= intersection.lon && intersection.lon <= first.lon)
                {
                    intersections += 1;
                }
            }
        }
        self.is_water = intersections % 2 == 0;
    }
}

fn calculate_bearing(first: &Coordinate, second: &Coordinate) -> f64 {
    f64::atan2(
        (first.get_lon() - second.get_lon()).abs().sin() * second.get_lat().cos(),
        first.get_lon().cos() * second.get_lon().sin()
            - first.get_lat().sin()
                * second.get_lat().cos()
                * (first.get_lon() - second.get_lon()).abs().cos(),
    )
}

fn calculate_intersection(first: &Coordinate, second: &Coordinate, bearing: f64) -> Coordinate {
    let angular_dist_1_2 = 2.0
        * f64::asin(f64::sqrt(
            ((first.get_lat() - second.get_lat()) / 2.0)
                .abs()
                .sin()
                .powi(2)
                + first.get_lat().cos()
                    * second.get_lat().cos()
                    * ((first.get_lon() - second.get_lon()) / 2.0)
                        .abs()
                        .sin()
                        .powi(2),
        ));

    let bearing_a = f64::acos(
        (second.get_lat().sin() - first.get_lat().sin() * angular_dist_1_2.cos())
            / (angular_dist_1_2.sin() * first.get_lat().cos()),
    );
    let bearing_b = f64::acos(
        (first.get_lat().sin() - second.get_lat().sin() * angular_dist_1_2.cos())
            / (angular_dist_1_2.sin() * second.get_lat().cos()),
    );

    let bearing_1_2;
    let bearing_2_1;
    if f64::sin(second.get_lon() - first.get_lon()) > 0.0 {
        bearing_1_2 = bearing_a;
        bearing_2_1 = 2.0 * std::f64::consts::PI - bearing_b;
    } else {
        bearing_1_2 = 2.0 * std::f64::consts::PI - bearing_a;
        bearing_2_1 = bearing_b;
    }

    let angle_1 = -bearing_1_2;
    let angle_2 = bearing_2_1 - bearing;
    let angle_3 = f64::acos(
        -angle_1.cos() * angle_2.cos() + angle_1.sin() * angle_2.sin() * angular_dist_1_2.cos(),
    );

    let angular_dist_1_3 = f64::atan2(
        angular_dist_1_2.sin() * angle_1.sin() * angle_2.sin(),
        angle_2.cos() + angle_1.cos() * angle_3.cos(),
    );

    let lat = f64::asin(
        first.get_lat().sin() * angular_dist_1_3.cos()
            + first.get_lat().cos() * angular_dist_1_3.sin(),
    );

    let lon_1_3 = std::f64::consts::PI / 2.0;

    let lon = first.get_lon() + lon_1_3;

    Coordinate {
        lon: (lon * FACTOR) as i32,
        lat: (lat * FACTOR) as i32,
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
    }

    let mut nodes = Node::generate_nodes();
    for node in nodes.iter_mut() {
        node.set_water_flag(&coasts);
    }

    Ok(())
}
