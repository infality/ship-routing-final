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
struct GEOJson<T> {
    r#type: &'static str,
    features: Vec<GEOJsonFeature<T>>,
}

#[derive(serde::Serialize)]
struct GEOJsonFeature<T> {
    r#type: &'static str,
    geometry: GEOJsonGeometry<T>,
    properties: GEOJsonProperty,
}

#[derive(serde::Serialize)]
struct GEOJsonGeometry<T> {
    r#type: &'static str,
    coordinates: T,
}

#[derive(serde::Serialize)]
struct GEOJsonProperty {}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
struct Coordinate {
    lat: i32,
    lon: i32,
}

impl Coordinate {
    fn is_equal(&self, other: &Coordinate) -> bool {
        return self.lon == other.lon && self.lat == other.lat;
    }

    fn get_lat(&self) -> f64 {
        to_radians(self.lat as f64 / FACTOR)
    }

    fn get_lon(&self) -> f64 {
        to_radians(self.lon as f64 / FACTOR)
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
                        lat: n.decimicro_lat,
                        lon: n.decimicro_lon,
                    },
                )),
                Ok(osmpbfreader::OsmObj::Way(w)) => {
                    let mut coordinates = Vec::<Coordinate>::with_capacity(w.nodes.len());
                    let mut leftmost = i32::MAX;
                    let mut rightmost = i32::MIN;
                    for node in w.nodes.iter() {
                        let n = nodes.get(&node.0).unwrap().clone();
                        coordinates.push(n);
                        if n.lon < leftmost {
                            leftmost = n.lon;
                        }
                        if n.lon > rightmost {
                            rightmost = n.lon;
                        }
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
    fn set_water_flag(&mut self, coasts: &Coasts) {
        for coast in coasts.actual_coasts.iter() {
            if self.coordinate.lon < coast.leftmost || self.coordinate.lon > coast.rightmost {
                continue;
            }

            let mut intersections = 0;
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
            if intersections % 2 == 1 {
                println!("X");
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

        // TODO Generate equally distributed nodes
        for lon in 6..=7 {
            for lat in 4..=5 {
                nodes.push(Node {
                    coordinate: Coordinate {
                        lat: lat * FACTOR as i32,
                        lon: lon * FACTOR as i32,
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

        // TODO Generate equally distributed nodes
        for lon in (0..(180 * 1)).step_by(1) {
            for lat in (0..(90 * 1)).step_by(1) {
                nodes.push(Node {
                    coordinate: Coordinate {
                        lat: lat * 10000000,
                        lon: lon * 10000000,
                    },
                    is_water: false,
                });
                if lon != 0 {
                    nodes.push(Node {
                        coordinate: Coordinate {
                            lat: lat * 10000000,
                            lon: -lon * 10000000,
                        },
                        is_water: false,
                    });
                }
                if lat != 0 {
                    nodes.push(Node {
                        coordinate: Coordinate {
                            lat: -lat * 10000000,
                            lon: lon * 10000000,
                        },
                        is_water: false,
                    });
                }
                if lon != 0 && lat != 0 {
                    nodes.push(Node {
                        coordinate: Coordinate {
                            lat: -lat * 10000000,
                            lon: -lon * 10000000,
                        },
                        is_water: false,
                    });
                }
            }
        }

        Nodes { nodes }
    }

    fn write_to_geojson(&self, filename: &str) {
        println!("Saving Nodes to geojson file: {}", filename);
        let mut geo_json = GEOJson {
            r#type: "FeatureCollection",
            features: Vec::new(),
        };

        for node in self.nodes.iter() {
            if node.is_water {
                continue;
            }
            let coordinates = [
                node.coordinate.lat as f64 / 10000000f64,
                node.coordinate.lon as f64 / 10000000f64,
            ];

            geo_json.features.push(GEOJsonFeature {
                r#type: "Feature",
                geometry: GEOJsonGeometry {
                    r#type: "Point",
                    coordinates,
                },
                properties: GEOJsonProperty {},
            });
        }

        let output_json = serde_json::to_string(&geo_json).unwrap();
        fs::write(&filename, output_json).unwrap();
    }
}

fn to_radians(value: f64) -> f64 {
    value * std::f64::consts::PI / 180.0
}

fn to_signed_degrees(value: f64) -> f64 {
    let result = value * 180.0 / std::f64::consts::PI;
    if result > 180.0 {
        return result - 360.0;
    }
    result
}

fn calculate_bearing(first: &Coordinate, second: &Coordinate) -> f64 {
    let bearing = f64::atan2(
        (second.get_lon() - first.get_lon()).sin() * second.get_lat().cos(),
        first.get_lat().cos() * second.get_lat().sin()
            - first.get_lat().sin()
                * second.get_lat().cos()
                * (second.get_lon() - first.get_lon()).cos(),
    );
    (bearing + 2.0 * std::f64::consts::PI) % (2.0 * std::f64::consts::PI)
}

fn calculate_intersection(first: &Coordinate, second: &Coordinate, bearing: f64) -> Coordinate {
    let angular_dist_1_2 = 2.0
        * f64::asin(f64::sqrt(
            ((second.get_lat() - first.get_lat()) / 2.0).sin().powi(2)
                + first.get_lat().cos()
                    * second.get_lat().cos()
                    * ((second.get_lon() - first.get_lon()) / 2.0).sin().powi(2),
        ));

    let cos_bearing_a = (second.get_lat().sin() - first.get_lat().sin() * angular_dist_1_2.cos())
        / (angular_dist_1_2.sin() * first.get_lat().cos());
    let cos_bearing_b = (first.get_lat().sin() - second.get_lat().sin() * angular_dist_1_2.cos())
        / (angular_dist_1_2.sin() * second.get_lat().cos());

    // Protect against rounding errors
    let bearing_a = f64::acos(f64::min(f64::max(cos_bearing_a, -1.0), 1.0));
    let bearing_b = f64::acos(f64::min(f64::max(cos_bearing_b, -1.0), 1.0));

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

    if angle_1.sin() == 0.0 && angle_2.sin() == 0.0 {
        panic!("infinite intersections");
    }
    if angle_1.sin() * angle_2.sin() < 0.0 {
        //panic!("ambiguous intersection (antipodal?)");
    }

    let cos_angle_3 =
        -angle_1.cos() * angle_2.cos() + angle_1.sin() * angle_2.sin() * angular_dist_1_2.cos();

    let angular_dist_1_3 = f64::atan2(
        angular_dist_1_2.sin() * angle_1.sin() * angle_2.sin(),
        angle_2.cos() + angle_1.cos() * cos_angle_3,
    );

    let lat = f64::asin(f64::min(
        f64::max(
            first.get_lat().sin() * angular_dist_1_3.cos()
                + first.get_lat().cos() * angular_dist_1_3.sin(),
            -1.0,
        ),
        1.0,
    ));

    let delta_lon_1_3 = f64::atan2(
        0.0,
        angular_dist_1_3.cos() - first.get_lat().sin() * lat.sin(),
    );

    let lon = first.get_lon() + delta_lon_1_3;

    //println!("{} | {}", to_signed_degrees(lat), to_signed_degrees(lon));
    Coordinate {
        lat: (to_signed_degrees(lat) * FACTOR) as i32,
        lon: (to_signed_degrees(lon) * FACTOR) as i32,
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

    let mut nodes = Nodes::new_generate_not_equally_distributed();
    //let mut nodes = Nodes::new_generate_equally_distributed();

    let mut counter = 0;
    let node_count = nodes.nodes.len();
    for node in nodes.nodes.iter_mut() {
        if counter % 1000 == 0 {
            println!("Setting water flags: {}/{}", counter, node_count);
        }
        counter += 1;
        node.set_water_flag(&coasts);
    }

    nodes.write_to_geojson("nodes.json");

    Ok(())
}

#[test]
fn test_bearing() {
    assert_eq!(
        calculate_bearing(
            &Coordinate {
                lat: 500000000,
                lon: 500000000,
            },
            &Coordinate {
                lat: 500000000,
                lon: 1000000000,
            }
        ),
        to_radians(70.342778)
    );
}

#[test]
fn test_intersection() {
    assert_eq!(
        calculate_intersection(
            &Coordinate {
                lat: 500000000,
                lon: 500000000,
            },
            &Coordinate {
                lat: 500000000,
                lon: 1000000000,
            },
            to_radians(32.44)
        ),
        Coordinate {
            lat: 479577780,
            lon: -1300000000,
        }
    );
}
