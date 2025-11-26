use crate::consts::DEFAULT_MAP_ID;

#[derive(Debug, Clone)]
pub enum MapIdentifier {
    Id(usize),
    Name(String),
    Random,
}

pub struct Flags {
    pub map: MapIdentifier,
    pub specific_map: bool,
    pub permanent_map: bool,
    pub random_map: bool,
    pub rand_map_side: Option<usize>,
}

pub fn parse_flags<I>(args: I) -> Option<Flags>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    iter.next();

    let mut map = MapIdentifier::Id(DEFAULT_MAP_ID);
    let mut specific_map = false;
    let mut permanent_map = false;
    let mut random_map = false;
    let mut rand_map_side = None;
    let args: Vec<String> = iter.collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-m" | "--map" => {
                specific_map = true;
                if i + 1 < args.len() {
                    let value = &args[i + 1];
                    // Try parsing as usize first
                    if let Ok(id) = value.parse::<usize>() {
                        if id > 0 {
                            map = MapIdentifier::Id(id);
                            i += 2;
                            continue;
                        }
                    }
                    // If parsing as usize fails, treat it as a map name
                    map = MapIdentifier::Name(value.clone());
                    i += 2;
                    continue;
                }
            }
            "-pm" | "--permanent-map" => {
                permanent_map = true;
                i += 1;
                continue;
            }
            "-rm" | "--random-map" => {
                random_map = true;
                // Check if length
                if i + 1 < args.len() {
                    if let Ok(side) = args[i + 1].parse::<usize>() {
                        if side < 4 || side > 100 {
                            println!(
                                "Error: Random map side length must be between 4 and 100 (got {})",
                                side
                            );
                            return None;
                        }
                        // Do not allow maps with more data than 35x35
                        if side > 35 {
                            println!(
                                "Error: Total random map side length must be less than 35, but got {}",
                                side
                            );
                            return None;
                        }
                        rand_map_side = Some(side);
                        i += 2;
                        continue;
                    } else {
                        println!(
                            "Error: --random-map requires a valid number (side length) if dimensions are provided"
                        );
                        return None;
                    }
                } else {
                    // No numbers provided - use defaults (None)
                    i += 1;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }

    if random_map && specific_map {
        println!("Error: Cannot use both --random-map and --map at the same time");
        return None;
    }

    Some(Flags {
        map,
        specific_map,
        permanent_map,
        random_map,
        rand_map_side,
    })
}
