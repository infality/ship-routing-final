use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::BufReader,
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

struct Coast {
    used: bool,
    coordinates: Vec<Coordinate>,
}

#[derive(Clone, Copy)]
struct Coordinate {
    lon: f64,
    lat: f64,
}

impl Coordinate {
    fn is_equal(&self, other: &Coordinate) -> bool {
        return self.lon == other.lon && self.lat == other.lat;
    }
}

struct ActualCoast {
    start: Coordinate,
    last: Coordinate,
    coordinates: Vec<Coordinate>,
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Please pass a pbf file");
    }

    let file = File::open(&args[1])?;
    let reader = BufReader::new(file);

    let mut pbf = osmpbfreader::OsmPbfReader::new(reader);

    let mut nodes = HashMap::<i64, Coordinate>::with_capacity(63000000);
    let mut coasts = Vec::<Coast>::with_capacity(1000000);

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
                    lon: n.lon(),
                    lat: n.lat(),
                },
            )),
            Ok(osmpbfreader::OsmObj::Way(w)) => {
                let mut coordinates = Vec::<Coordinate>::with_capacity(w.nodes.len());
                for node in w.nodes.iter() {
                    coordinates.push(nodes.get(&node.0).unwrap().clone());
                }
                coasts.push(Coast {
                    used: false,
                    coordinates,
                });
            }
            _ => continue,
        }
    }

    println!("Found {} nodes", nodes.len());
    println!("Found {} ways", coasts.len());
    println!("Finished parsing");

    let mut actual_coasts = Vec::<ActualCoast>::new();
    let mut current_coast;
    {
        let first_coast = coasts.first_mut().unwrap();
        first_coast.used = true;
        current_coast = ActualCoast {
            start: first_coast.coordinates.first().unwrap().clone(),
            last: first_coast.coordinates.last().unwrap().clone(),
            coordinates: first_coast.coordinates.clone(),
        };
    }

    counter = 0;
    loop {
        println!("Merging coasts: {}", counter);
        counter += 1;

        while !current_coast.start.is_equal(&current_coast.last) {
            for coast in coasts.iter_mut() {
                if coast.used {
                    continue;
                }

                if current_coast
                    .last
                    .is_equal(coast.coordinates.first().unwrap())
                {
                    coast.used = true;
                    current_coast.last = coast.coordinates.last().unwrap().clone();
                    current_coast.coordinates.append(&mut coast.coordinates);
                } else {
                    continue;
                }
                break;
            }
        }

        actual_coasts.push(current_coast);

        let next_coast = coasts.iter_mut().find(|c| !c.used);
        if next_coast.is_none() {
            break;
        }
        let next_coast = next_coast.unwrap();
        next_coast.used = true;

        current_coast = ActualCoast {
            start: next_coast.coordinates.first().unwrap().clone(),
            last: next_coast.coordinates.last().unwrap().clone(),
            coordinates: next_coast.coordinates.clone(),
        };
    }

    println!("Found {} actual coasts", actual_coasts.len());
    println!("Finished merging");

    let mut geo_json = GEOJson {
        r#type: "FeatureCollection",
        features: Vec::new(),
    };

    for actual_coast in actual_coasts.iter() {
        let mut coordinates = Vec::<[f64; 2]>::new();

        for coordinate in actual_coast.coordinates.iter().rev() {
            coordinates.push([coordinate.lon, coordinate.lat]);
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

    let output_json = serde_json::to_string(&geo_json)?;

    fs::write("coastlines.json", output_json)?;

    Ok(())
}
