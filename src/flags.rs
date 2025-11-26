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
    pub rand_map_width: Option<usize>,
    pub rand_map_height: Option<usize>,
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
    let mut rand_map_width = None;
    let mut rand_map_height = None;
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
                // Check if width and height are provided
                if i + 2 < args.len() {
                    // Try to parse both numbers
                    if let (Ok(width), Ok(height)) =
                        (args[i + 1].parse::<usize>(), args[i + 2].parse::<usize>())
                    {
                        // Validate both numbers are between 4 and 100
                        if width < 4 || width > 100 {
                            println!(
                                "Error: Random map width must be between 4 and 100 (got {})",
                                width
                            );
                            return None;
                        }
                        if height < 4 || height > 100 {
                            println!(
                                "Error: Random map height must be between 4 and 100 (got {})",
                                height
                            );
                            return None;
                        }
                        // Do not allow maps with more data than 35x35
                        if width * height > 1225 {
                            println!("Error: Total random map size must be less than 35x35 (which is 1225 tiles), but got {}x{}, which is {} tiles", width, height, width * height);
                            return None;
                        }
                        rand_map_width = Some(width);
                        rand_map_height = Some(height);
                        i += 3;
                        continue;
                    } else {
                        println!(
                            "Error: --random-map requires two valid numbers (width height) if dimensions are provided"
                        );
                        return None;
                    }
                } else if i + 1 < args.len() {
                    // Only one number provided - error
                    println!(
                        "Error: --random-map requires both width and height, or neither. Only one number provided."
                    );
                    return None;
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
        rand_map_width,
        rand_map_height,
    })
}
