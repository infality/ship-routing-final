use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::BufReader,
    io::BufWriter,
    io::Error,
};

const FACTOR: f64 = 10_000_000.0;
const WATER: Coordinate = Coordinate {
    lat: 900000000,
    lon: 0,
};

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

        //let actual_coast = &self.actual_coasts[1];
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
        let mut i = 0;
        let mut nodes_intersections = Nodes { nodes: vec![] };
        for coast in coasts.actual_coasts.iter() {
            if self.coordinate.lon < coast.leftmost || self.coordinate.lon > coast.rightmost {
                continue;
            }

            let mut intersection_count = 0;
            for line in 0..coast.coordinates.len() {
                let first = coast.coordinates[line];
                let second = coast.coordinates[(line + 1) % coast.coordinates.len()];

                if (first.lon <= self.coordinate.lon && self.coordinate.lon <= second.lon)
                    || (second.lon <= self.coordinate.lon && self.coordinate.lon <= first.lon)
                {
                } else {
                    continue;
                }

                // Handle special case if line is vertical
                if first.lon == second.lon {
                    if first.lon == self.coordinate.lon
                        && i32::max(first.lat, second.lat) >= self.coordinate.lat
                    {
                        // Check orientation of surrounding non-vertical coast lines and decide if
                        // it counts as an intersection
                        let mut is_prev_right = false;
                        let mut is_next_right = false;

                        for prev_coordinate in 0..(coast.coordinates.len() - 2) {
                            let prev = coast.coordinates
                                [(line - prev_coordinate) % coast.coordinates.len()];
                            if prev.lon != self.coordinate.lon {
                                is_prev_right = prev.lon > self.coordinate.lon;
                                break;
                            }
                        }

                        for next_coordinate in 2..coast.coordinates.len() {
                            let next = coast.coordinates
                                [(line + next_coordinate) % coast.coordinates.len()];
                            if next.lon != self.coordinate.lon {
                                is_next_right = next.lon > self.coordinate.lon;
                                break;
                            }
                        }

                        if is_prev_right != is_next_right {
                            intersection_count += 1;
                        }
                    }
                    continue;
                }

                let intersections =
                    calculate_intersections(&self.coordinate, &WATER, &first, &second);
                nodes_intersections.nodes.push(Node {
                    coordinate: intersections,
                    is_water: false,
                });

                println!("{}, {}", intersections.get_lon(), intersections.get_lat());
                // Check if the intersection is on the coast line
                if (first.lon <= intersections.lon && intersections.lon <= second.lon)
                    || (second.lon <= intersections.lon && intersections.lon <= first.lon)
                {
                    intersection_count += 1;
                    //println!("yes")
                } else {
                    //println!("nope")
                }
            }
            //println!("{}", intersection_count);
            if intersection_count % 2 == 1 {
                //println!("node is inside coastline-polygon {}", i);
                self.is_water = false;
                return;
            }
            i = i + 1;
        }
        //nodes_intersections.write_to_geojson("intersections.json");
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
                        lon: lon * FACTOR as i32,
                        lat: lat * FACTOR as i32,
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
        for lon in (0..(180 * 1 + 1)).step_by(10) {
            for lat in (0..(90 * 1 + 1)).step_by(10) {
                nodes.push(Node {
                    coordinate: Coordinate {
                        lon: lon * 10000000,
                        lat: lat * 10000000,
                    },
                    is_water: true,
                });
                if lon != 0 {
                    nodes.push(Node {
                        coordinate: Coordinate {
                            lon: -lon * 10000000,
                            lat: lat * 10000000,
                        },
                        is_water: true,
                    });
                }
                if lat != 0 {
                    nodes.push(Node {
                        coordinate: Coordinate {
                            lon: lon * 10000000,
                            lat: -lat * 10000000,
                        },
                        is_water: true,
                    });
                }
                if lon != 0 && lat != 0 {
                    nodes.push(Node {
                        coordinate: Coordinate {
                            lon: -lon * 10000000,
                            lat: -lat * 10000000,
                        },
                        is_water: true,
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
            if !node.is_water {
                continue;
            }
            let coordinates = [
                node.coordinate.lon as f64 / 10000000f64,
                node.coordinate.lat as f64 / 10000000f64,
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

fn to_degrees(value: f64) -> f64 {
    value * 180.0 / std::f64::consts::PI
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn calculate_intersections(
    p1: &Coordinate,
    p2: &Coordinate,
    p3: &Coordinate,
    p4: &Coordinate,
) -> Coordinate {
    let p1_lon_rad = to_radians(p1.get_lon());
    let p1_lat_rad = to_radians(p1.get_lat());
    let p2_lon_rad = to_radians(p2.get_lon());
    let p2_lat_rad = to_radians(p2.get_lat());
    let p3_lon_rad = to_radians(p3.get_lon());
    let p3_lat_rad = to_radians(p3.get_lat());
    let p4_lon_rad = to_radians(p4.get_lon());
    let p4_lat_rad = to_radians(p4.get_lat());

    // Convert to vector
    let v1_x = p1_lat_rad.cos() * p1_lon_rad.cos();
    let v1_y = p1_lat_rad.cos() * p1_lon_rad.sin();
    let v1_z = p1_lat_rad.sin();
    let v2_x = p2_lat_rad.cos() * p2_lon_rad.cos();
    let v2_y = p2_lat_rad.cos() * p2_lon_rad.sin();
    let v2_z = p2_lat_rad.sin();
    let v3_x = p3_lat_rad.cos() * p3_lon_rad.cos();
    let v3_y = p3_lat_rad.cos() * p3_lon_rad.sin();
    let v3_z = p3_lat_rad.sin();
    let v4_x = p4_lat_rad.cos() * p4_lon_rad.cos();
    let v4_y = p4_lat_rad.cos() * p4_lon_rad.sin();
    let v4_z = p4_lat_rad.sin();

    // Get great-circles
    let gc1 = cross([v1_x, v1_y, v1_z], [v2_x, v2_y, v2_z]);
    let gc2 = cross([v3_x, v3_y, v3_z], [v4_x, v4_y, v4_z]);

    // Get intersection points
    let c1 = cross(gc1, gc2);
    let c2 = cross(gc2, gc1);

    // Find nearest intersection and convert back to lat/lon
    let mid = dot(
        [
            v1_x + v2_x + v3_x + v4_x,
            v1_y + v2_y + v3_y + v4_y,
            v1_z + v2_z + v3_z + v4_z,
        ],
        c1,
    );
    if mid > 0.0 {
        let lat1 = to_degrees(f64::atan2(c1[2], f64::sqrt(c1[0].powi(2) + c1[1].powi(2))));
        let lon1 = to_degrees(f64::atan2(c1[1], c1[0]));
        Coordinate {
            lon: (lon1 * FACTOR) as i32,
            lat: (lat1 * FACTOR) as i32,
        }
    } else {
        let lat2 = to_degrees(f64::atan2(c2[2], f64::sqrt(c2[0].powi(2) + c2[1].powi(2))));
        let lon2 = to_degrees(f64::atan2(c2[1], c2[0]));
        Coordinate {
            lon: (lon2 * FACTOR) as i32,
            lat: (lat2 * FACTOR) as i32,
        }
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

    let mut coasts;
    if !skip_read_pbf {
        coasts = Coasts::new_from_pbffile(&file_name);
        coasts.write_to_geojson("coastlines.json");
        coasts.write_to_binfile("coastlines.bin");
    } else {
        coasts = Coasts::new_from_binfile(&file_name);
        //coasts.write_to_geojson("coastlines.json");
    }
    coasts = Coasts {
        actual_coasts: vec![Coast {
            coordinates: vec![
                Coordinate {
                    lon: -100000000,
                    lat: 340000000,
                },
                Coordinate {
                    lon: 570000000,
                    lat: 490000000,
                },
                Coordinate {
                    lon: 126000000,
                    lat: 200000000,
                },
                Coordinate {
                    lon: 380000000,
                    lat: -110000000,
                },
            ],
            leftmost: -100000000,
            rightmost: 126000000,
        }],
    };
    coasts.write_to_geojson("coastlines-custom.json");

    let mut nodes = Nodes::new_generate_not_equally_distributed();
    nodes.write_to_geojson("grid.json");

    //let mut nodes = Nodes::new_generate_equally_distributed();

    let mut counter = 0;
    let node_count = nodes.nodes.len();
    for node in nodes.nodes.iter_mut() {
        if counter % 1000 == 0 {
            println!("Setting water flags: {}/{}", counter, node_count);
        }
        counter += 1;
        node.set_water_flag(&coasts);
        //node.set_water_flag_spherical(&coasts);
    }

    nodes.write_to_geojson("nodes.json");

    Ok(())
}

#[test]
fn test_intersections() {
    /* assert_eq!(
        calculate_intersections(
            &Coordinate {
                lat: 330000000,
                lon: -240000000,
            },
            &Coordinate {
                lat: 520000000,
                lon: -700000000,
            },
            &Coordinate {
                lat: 490000000,
                lon: 10000000,
            },
            &Coordinate {
                lat: 460000000,
                lon: 150000000,
            },
        ),
        (
            Coordinate {
                lat: -467105560,
                lon: 1668375000,
            },
            Coordinate {
                lat: 467105560,
                lon: -131625000,
            }
        )
    ); */
}
