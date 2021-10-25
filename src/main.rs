use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::BufReader,
    io::BufWriter,
    io::Error,
};


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


#[derive(Clone, Copy, Hash, Eq, PartialEq)]
#[derive(serde::Serialize)]
#[derive(serde::Deserialize)]
struct Coordinate {
    lon: i32,
    lat: i32,
}

impl Coordinate {
    fn is_equal(&self, other: &Coordinate) -> bool {
        return self.lon == other.lon && self.lat == other.lat;
    }
}


#[derive(serde::Serialize)]
#[derive(serde::Deserialize)]
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


#[derive(serde::Serialize)]
#[derive(serde::Deserialize)]
struct Coasts {
    actual_coasts: Vec::<Coast>,
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
            while !current_coast.get_first().is_equal(&current_coast.get_last()) {
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
        return Coasts {
            actual_coasts: actual_coasts,
        }
    }

    fn new_from_binfile(filename: &str) -> Self {
        println!("Creating Coasts from bin file: {}", filename);
        let mut buf_reader = BufReader::new(File::open(&filename).unwrap());
        let coasts:Self = bincode::deserialize_from(&mut buf_reader).unwrap();
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
                coordinates.push([
                    coordinate.lon as f64 / 10000000f64,
                    coordinate.lat as f64 / 10000000f64,
                ]);
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

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Please pass a pbf file");
    }

    let coasts = Coasts::new_from_pbffile(&args[1]);

    coasts.write_to_geojson("coastlines.json");
    coasts.write_to_binfile("coastlines.bin");

    let _coasts = Coasts::new_from_binfile("coastlines.bin");

    Ok(())
}
